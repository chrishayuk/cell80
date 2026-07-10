# Sliding-window state cells — the array-state-field gap, confirmed

Status: **not an experiment with a hypothesis — a debugging finding from trying to actually
land `simple_moving_average`.** `docs/library-growth.md` and the running-stats pack README
had flagged "the array-state-field question" as open since wave 3, but no one had actually hit
the wall and named it precisely. This session did, while working through the roadmap's
library-growth track (`docs/library-growth.md` "Next waves", the sliding-window family).

## The claim that turned out false

The plan was: a fixed-window moving average needs a ring buffer (`window: [u16; 8]`) plus
`head`/`count`/`sum` scalars, all as struct fields on a state cell — array struct fields are
already a real, tested compiler feature (`self.arr[i]` read/write through `self`, used by the
`chase` game and covered in `rustz80/tests/diff/{inline,peephole,nested_structs,structs}.rs`),
so this looked like pure authoring, not new capability. It isn't: the compiler support was
never the gap.

## Root cause

`Runner::exec` (`cell80/src/runner.rs:721-727`) zeros every byte the *previous* run touched,
before applying this run's inputs:

```rust
// Reset only the bytes the previous run wrote, then restore the code (in case it
// was poked).
for &a in &self.touched {
    self.mem[a as usize] = 0;
    self.seen[a as usize] = false;
}
self.touched.clear();
```

So a "state cell" has no real persistent memory across separate `run()` calls — the entire
illusion of state is the host re-supplying the previous outcome as this call's input. That's
exactly why `accumulate_step`'s own host-oracle test (`cell80/tests/library/running-stats.rs`)
re-sets `sum`/`count` from the prior call's output before every iteration, rather than just
calling `run()` in a loop.

The named-field round-trip surface that makes this practical — `StateCell::set`/`get`
(`cell80/src/state.rs`) and `CellHost::run_state`/`run_state_fast` (`cell80/src/host.rs`) — only
knows **scalar** fields. `state.rs::scalar_ty` maps a `FieldLayout` to `Ty::U8/U16/U32/F32`
and returns `None` for anything else, which silently includes array fields (`f.slots > 1`,
not `dword`, not `bytes`) — they're laid out and reachable *inside* the compiled cell, but
invisible to every host-facing round-trip. This is the same shape as the already-declared,
never-built "Phase S3" byte-buffer I/O for `bytes[N]`/`str[N]` (`cell80/src/report.rs`,
`cell80/src/runner.rs:457-465`) — both are "an array of same-width slots the host needs to
write and read back by name," just at different element widths (bytes vs. words).

## What was actually verified

`experiments/sliding-window-state-cells/simple_moving_average.rs` (below) — a true trailing
8-sample window, distinct from `accumulate_step`/`running_variance_step` (cumulative over the
whole stream, never forgets a sample):

```rust
//! Simple moving average over a fixed trailing window of the last 8 values — a true sliding window (subtracts the value leaving the window each step), distinct from accumulate_step/running_variance_step which are cumulative over the whole stream and never forget a sample. Self-initializing: the average is over however many samples have arrived until the window fills, then always over exactly the last 8.
//! tags: moving, average, sma, window, sliding, rolling, trailing, stream, state, wide
//! entry: SimpleMovingAverage::run
//! limits: fixed 8-sample trailing window, not caller-configurable; the divisor is min(samples_seen, 8), so it's never zero
struct SimpleMovingAverage { value: u16, window: [u16; 8], head: u16, count: u16, sum: u32, avg: u16 }
impl SimpleMovingAverage {
    fn run(&mut self) -> u16 {
        let full = self.count == 8u16;
        let evict = if full { self.window[self.head as usize] as u32 } else { 0u32 };
        self.window[self.head as usize] = self.value;
        self.sum = self.sum - evict + (self.value as u32);
        if !full { self.count = self.count + 1u16; }
        self.head = (self.head + 1u16) % 8u16;
        self.avg = (self.sum / (self.count as u32)) as u16;
        self.avg
    }
}
```

First attempt drove it through `StateCell` exactly like `accumulate_step` (re-set every named
scalar each call) and failed on the *second* call (`value=20` returned `avg=20`, not `15`) —
because `window` never round-tripped, so the previous call's write vanished with the reset and
every run effectively restarted from `count=0`.

Verified correct — including the compiler's array-field layout (`value@0, window@1..8 (8
slots), head@9, count@10, sum@11 (dword, 2 slots), avg@13`, confirmed via
`rustz80::struct_layout`, no overlap) — via a raw-address round trip that bypasses `StateCell`
entirely and manually re-feeds *every* field, window elements included, through
`Runner::run_with_inputs`'s `(addr, Ty, value)` triples:

```rust
use cell80::{Runner, Ty, DEFAULT_CYCLES};

const STATE_BASE: u16 = 0xB000;
const VALUE: u16 = STATE_BASE;
const WINDOW0: u16 = STATE_BASE + 2;
const HEAD: u16 = STATE_BASE + 18;
const COUNT: u16 = STATE_BASE + 20;
const SUM: u16 = STATE_BASE + 22;
const AVG: u16 = STATE_BASE + 26;

#[test]
fn probe_sma_full_state_roundtrip() {
    let src = std::fs::read_to_string("simple_moving_average.rs").unwrap();
    let mut r = Runner::compile(&src).unwrap();

    let mut window = [0u16; 8];
    let mut head: u16 = 0;
    let mut count: u16 = 0;
    let mut sum: u32 = 0;

    let mut step = |r: &mut Runner, value: u16| -> u16 {
        let mut inputs: Vec<(u16, Ty, u64)> = vec![
            (VALUE, Ty::U16, value as u64),
            (HEAD, Ty::U16, head as u64),
            (COUNT, Ty::U16, count as u64),
            (SUM, Ty::U32, sum as u64),
        ];
        for (i, w) in window.iter().enumerate() {
            inputs.push((WINDOW0 + (i as u16) * 2, Ty::U16, *w as u64));
        }
        let report = r
            .run_with_inputs(Some("SimpleMovingAverage::run"), &[STATE_BASE], &inputs, DEFAULT_CYCLES)
            .unwrap();
        head = r.peek_u16(HEAD);
        count = r.peek_u16(COUNT);
        sum = r.peek_u32(SUM);
        for (i, w) in window.iter_mut().enumerate() {
            *w = r.peek_u16(WINDOW0 + (i as u16) * 2);
        }
        let avg = r.peek_u16(AVG);
        assert_eq!(report.result, avg);
        avg
    };

    // Filling the window (avg over samples-seen-so-far), then past the fill point (the
    // oldest sample leaves the window as each new one arrives): 10..80 fill exactly, then
    // 90 evicts 10 (avg 55), 100 evicts 20 (avg 65).
    for (value, want) in [
        (10, 10), (20, 15), (30, 20), (40, 25),
        (50, 30), (60, 35), (70, 40), (80, 45),
        (90, 55), (100, 65),
    ] {
        assert_eq!(step(&mut r, value), want, "value={value}");
    }
}
```

Both files ran clean in an isolated `git worktree` off the last committed `main` (a parallel
session had `rustz80/src` mid-edit for the multi-target A2/A3 work at the time — verifying
this way kept the finding real without touching or depending on their in-flight state).

## Why the cell wasn't landed

Shipping `simple_moving_average.rs` into `cell80/cells/running-stats/` now would pass the
admission gate (behaviourally distinct, has retrieval rows) but silently misbehave for any real
consumer — `StateCell`, `cell80-py`, `cell80-mcp`'s `cell_run(fields=…)` all drive state cells
by named scalar field, and none of them can write `window` back in. An agent calling this
through the standard MCP path would get a plausible-looking wrong `avg` after the first sample,
with no error — exactly the class of silent-wrong-answer the whole project's admission
gate / cost-honesty / checked-arithmetic discipline exists to prevent. So the cell stays here,
verified but unlanded, until the round-trip surface exists.

## Open design questions for whoever builds the round-trip (fold into Phase S3, don't build twice)

`docs/09-cell80-abi.md`'s `bytes[N]`/`str[N]` buffer fields are declared (ABI v3) but their
host I/O was deferred to "Phase S3" and never built. A `[u16; N]` word array is the same shape
of problem at a different element width — one design should cover both rather than shipping an
array-specific surface now and reconciling it with S3 later. Questions this probe surfaced that
the design needs to answer:

1. **Element width.** `bytes[N]` is `u8`-only; a word array is `u16` (or, per the existing
   `wide_len` machinery, `u32`). Does one `Ty::Array(elem_ty, len)` variant cover both, or do
   they stay parallel (`Bytes(N)` / a new `Words(N)`)?
2. **Whole-envelope vs. logical-length.** Does a read return all `N` slots regardless of how
   many are semantically valid, or does the cell also need to expose a logical length (this
   cell already tracks that itself, as `count`, entirely inside the array-field problem — but a
   more general buffer type, e.g. a string, needs the host to know how much of the envelope is
   "real")?
3. **Per-field vs. whole-state blob.** The host already re-supplies *every* scalar field on
   every call (there's no partial persistence today) — so a per-field named array API
   (`set_array("window", &[...])`) buys ergonomics but not less data movement. A single
   "read/write the whole state region as bytes" primitive would be cheaper per call (one
   memcpy-shaped operation instead of N field lookups) and arguably matches how the host
   already treats state cells (round-trip everything, every time) more honestly than named
   per-field access does. Worth deciding which is primary and which (if either) rides on top.

Once that surface exists, `simple_moving_average` above should compile and pass unchanged
through the real `StateCell`/`CellHost::run_state` path — the cell logic is already verified;
only the round-trip is missing. `weighted_moving_average`, `rolling_variance`, and
`rolling_std` (the other math-server-mining candidates in this family,
`docs/math-server-map.md`) are blocked on the identical gate and should land as a batch right
behind it.
