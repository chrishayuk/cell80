# Growing the cell library — toward a large prebuilt collection

> **Before authoring a new cell, check [`docs/cell-index.md`](cell-index.md)** — every
> landed cell, grouped by pack, generated from the actual library (not hand-transcribed, so
> it can't drift the way this file's old cell-count prose once did). It's the fastest way to
> confirm a behaviour doesn't already exist before writing a duplicate the admission gate
> would just refuse.

*The goal is a **big, growing library of prebuilt cells** — hundreds of small, distinct,
deterministic integer utilities an agent can retrieve, run, and compose, organized into
**packs**. cell80's whole pitch is "millions of tiny tools, retrieved" — so the library should
be **broad**: the more genuinely-distinct behaviours sit on the shelf, the more an agent finds
one instead of writing code. This guide is how to grow it well.*

## The shape we're building toward

```
wave 1 ✓   59 cells   predicates · safe arithmetic · bounds · percent · ranking · bit/mask
wave 2 ✓   98 cells   + number theory · distance · bit/encoding · hashing · stats · conversion
wave 2.5    96 cells   + the wide u32-in-state siblings (square_wide, weighted_sum_wide);
                        4 folded into aliases by the admission gate (see below)
wave 3a   100 cells   + calendrical/checksum, first slice (is_leap_year, days_in_month,
                        day_of_week, luhn_check) — ISBN/IBAN/UPC deferred (see below)
wave 3b   103 cells   + Q8.8 fixed-point, first slice (q_mul, q_div, q_lerp) —
                        q_sqrt/piecewise sigmoid-tanh still open
wave 3c   108 cells   + agentic runtime primitives, first slice (token_bucket_step,
                        backoff_next, circuit_breaker_step, debounce_step, hysteresis) —
                        rate_window_update still open
wave 3d   111 cells   + running statistics, first slice (running_min_max_step,
                        streak_step, accumulate_step) — Welford variance still open
now       114 cells   + spatial/grid, first slice (grid_index, point_in_rect,
                        aabb_intersect) — Morton encode/decode, Bresenham still open
next      ~200+        + packing/BCD · vector · time/budget · stateful/RNG · signed deltas
```

All five originally-planned wave-3 packs (calendrical/checksum, fixed-point, agentic
runtime, running statistics, spatial/grid) have now landed a first slice; each deferred its
harder items (see the per-pack notes below and `docs/cell-index.md`'s "planned" section).

Cells are also **modular** now: a shared kernel prelude (`gcd`, `imin`, `imax`, `iabs_diff`,
`isqrt`, `clamp_to`) is appended to every cell and dead-code-eliminated, so `lcm` calls `gcd`
and `chebyshev` calls `iabs_diff`/`imax` instead of re-implementing them — and a cell that uses
no kernel stays byte-identical to having no prelude.

Big is the point — but **big and distinct**, not big and padded. Every cell earns its place by
being a *different behaviour*, not a renamed one.

## What a good cell is

> A tiny, deterministic utility an agent needs often but shouldn't spend tokens re-deriving.

```
small · deterministic · easy to test · easy to describe · useful in many workflows
cheap enough to run constantly · a distinct behaviour · part of a confusable family
```

## Two rules that keep a *large* library strong

A library that just accumulates functions rots in two ways — these rules prevent it:

1. **No behavioural duplicates — aliases live in metadata, not in code.** `time_until(now,
   deadline)` is `sub_sat`; `deadline_missed` is `is_ge`; inclusive `between` is `range_check`.
   Don't ship a second cell with the same behaviour — add the alias as a **tag/summary** on the
   existing one so search still finds it. Duplicates *hurt* retrieval (two right answers = no
   signal) and bloat the shelf without adding capability. **Enforced, not just requested:** the
   Phase 2.2 admission gate (`cell80 index --gate`, `cell80/src/admission.rs`) found four
   cells that had shipped as exact duplicates without anyone noticing — `argmin2` ≡ `is_gt`,
   `argmax2` ≡ `is_lt`, `quantize` ≡ `safe_div`, `wrap` ≡ `safe_mod`, the identical formula
   under a different name for every `u16` input. All four were removed and their vocabulary
   merged into the surviving cell's tags.
2. **Grow in confusable families, and pay the eval tax per cell.** Retrieval only gets *teeth*
   from 3-4+ cells per family that collide in text but differ in behaviour; composition needs
   predicates + transforms that chain. A new cell ships with its eval pressure or it's just
   inventory. See the contribution rule.

So "a large number of cells" and "good evals" pull the *same* direction: more distinct
confusable cells = a bigger shelf *and* a harder, more honest retrieval benchmark.

## Principles (what makes a cell worth adding)

- **Fits the integer envelope** (`u8`/`u16`/`u32`/`i16`, no float/string/syscall, bounded
  cycles). The compile error *is* the "this belongs in host code" signal. `i16` has landed
  (signed compare/divide/`>>`), so signed deltas are in-envelope; the unsigned abs-via-a-branch
  idiom (`abs_diff`/`manhattan`) remains fine where a cell doesn't need negatives.
- **≤ 3 args, or a state cell.** The calling convention takes 3 args (`HL`/`DE`/`BC`); a cell
  that needs more (4-point distance, multi-weight scoring) is a **state cell** (a `struct` with
  named fields + `fn run(&mut self)`), like `manhattan`/`chebyshev`.
- **Small and pure** — tens of bytes of behaviour, deterministic, cycle-honest.
- **Composes** — produces/consumes values others use; include **boolean predicates** (`-> 0/1`).

### What the compiler gives you (author cells clean)

- **Comparisons are values:** `fn run(a: u16, b: u16) -> u16 { (a < b) as u16 }`. All six.
- **`&&` / `||`** (short-circuit): `((lo < x) && (x < hi)) as u16` — and a short-circuit guard
  is the right way to keep a loop in-budget, e.g. `while r < 255 && (r+1)*(r+1) <= n { … }`.
- **Runtime bit shifts:** `x << bit` / `x >> bit` with a *variable* amount (a shift ≥ 16
  saturates a `u16` to `0`) — bit/rotate/encoding cells are one-liners.
- **Multi-function cells:** a cell may define helpers and call them; the entry is `run`
  (e.g. `lcm` calling a local `gcd`).

### Standardise these semantics

- **Predicate convention:** `false = 0`, `true = 1` (built on `bool as u16`).
- **Divide/remainder by zero:** unguarded `/` and `%` **halt the cell**
  (`Halt::DivByZero`, the Phase 0.3 default) rather than returning a value — **guard
  explicitly** when zero is a valid input (`if b != 0 { a / b } else { 0 }`); `safe_div`/
  `safe_mod` are canonical.
- **`u16` overflow is silent** (wraps); saturating cells (`add_sat`, `mul_sat`, `sum3`, …) cap
  at `65535`; percent/scale/`euclid_sq`/`triangular` assume their product fits `u16` (beyond is
  the host-code signal).

## The contribution rule (every new-cell PR)

```
1. cell80/cells/<name>.rs                       — header (//! summary, //! tags:) + fn/struct
2. cell-eval/datasets/retrieval.jsonl           — a direct row that ranks the cell #1
                                                   (verify with `cell80 search`), + paraphrase
3. composition or adoption task (if user-facing) — composition_tasks.jsonl / tasks.jsonl
4. cell80/tests/library.rs                       — edge-case rows (the host oracle)
5. docs/cell-index.md                            — regenerate (command at the top of the file)
```

Steps 1-2 are enforced, not just requested: `cell80 index cell80/cells --gate
cell-eval/datasets/retrieval.jsonl` (the Phase 2.2 admission gate, `cell80/src/admission.rs`)
refuses a candidate that's behaviourally identical to an already-shipped cell (alias it in
metadata instead) or that carries no retrieval rows to survive. Run it before opening a PR.

## Packs (organise discovery by family via tags)

The loader reads a flat `cell80/cells/`, so a "pack" is a **tag**, not a directory. Build them
out broadly:

```
math-core      bounds        percent       ranking-stats   number-theory   distance
bitops         bit-encoding  hashing       packing         time            budget
validation     vector        decimal       random/stateful scoring/choice  conversion
```

### Landed (114 cells)

See **[`docs/cell-index.md`](cell-index.md)** for the full, generated, per-pack list — not
duplicated here, so there's exactly one place this can go stale (and it's checked against
the real library every time it's regenerated).

**Calendrical / checksum, first slice (wave 3): `is_leap_year`, `days_in_month`,
`day_of_week`, `luhn_check`.** A real constraint surfaced authoring this pack: a free-fn
cell's calling convention is 16-bit registers, so **`u32` can only exist as a state field,
not a call param or return type** — a classic multi-digit checksum (Luhn over a real
13-19-digit card number, ISBN-13, IBAN mod-97) needs far more digits than even a `u32` state
field holds. `luhn_check` is scoped to a `u16` input (≤ 5 decimal digits, documented via
`//! limits:`) to stay a plain free function; ISBN-10/13, IBAN mod-97, and UPC are deferred
until either a state-cell version (carrying digits as array/state fields) or wider host-side
preprocessing is worth the design cost.

**Q8.8 fixed-point, first slice (wave 3): `q_mul`, `q_div`, `q_lerp`.** Sidesteps the same
`u32`-in-a-free-fn constraint by keeping params/return as `u16` and widening only as a
*local* (`a as u32 * b as u32`, `>> 8u32`) — the pattern any Q8.8 free function should
follow. `q_lerp` also serves as an EMA step (`q_lerp(prev, sample, alpha)`) — deliberately
*not* shipped as a second `q_ema` cell, since the formula is identical; the admission gate
would refuse it anyway. `q_sqrt` and piecewise `q_sigmoid`/`q_tanh` are still open.

**Agentic runtime primitives, first slice (wave 3): `token_bucket_step`, `backoff_next`,
`circuit_breaker_step`, `debounce_step`, `hysteresis`.** All genuinely need state (each
depends on outcomes from prior calls, not just this call's arguments), unlike the other
"time/budget" names already flagged in Next waves below — `used_percent`/`fits_budget`/
`cooldown_remaining` turned out to be aliases of `percent`/`is_le`/`sub_sat` respectively
and were never built, exactly the kind of check `docs/cell-index.md` is for. `backoff_next`
guards against a real overflow: doubling `current` directly can wrap past `u16::MAX` before
the cap check runs, so it compares against `cap / 2` first and only multiplies when doubling
is provably safe. `rate_window_update` is still open.

**Running statistics, first slice (wave 3): `running_min_max_step`, `streak_step`,
`accumulate_step`.** Deliberately doesn't reach for Welford's algorithm (which needs care in
fixed point) or a histogram (which needs array state fields, not yet exercised by any landed
cell) — instead `accumulate_step` keeps a running sum + count and composes with the
already-landed `safe_div` for the mean, rather than shipping a monolithic "running mean"
cell that would just re-implement `safe_div` internally. A fixed-point running variance and
percentile-from-histogram are still open, gated on that array-state-field question.

**Spatial / grid, first slice (wave 3): `grid_index`, `point_in_rect`, `aabb_intersect`.**
`grid_index` is a plain arity-3 free function; the other two are state cells purely for arg
count (6 and 8 named fields respectively), not width. Both containment checks are
half-open — edge-touching does not count as inside/overlapping, verified by hand for both.
Morton encode/decode were deliberately not attempted this slice: encoding a full `u16` x/y
pair needs a 32-bit interleaved result, so — like the calendrical/checksum pack's
discovery — it would need a `u32` state field, and the bit-interleaving loop itself
(computed shift amounts on a wide accumulator) hasn't been risked yet. A Bresenham line
stepper is also still open.

### Next waves (prioritized — keep them distinct)

- **packing / BCD**: `pack_u8`/`unpack_lo`/`unpack_hi`, `pack_nibbles`, `bcd_encode`/`bcd_decode`.
- **scoring / choice** (mostly state cells — need > 3 args): `weighted_sum2/3`, `score_2factor`,
  `choose_best3/4`, `is_clear_winner`, `tie_break_*`.
- **vector** (state cells): `dot2`, `norm2_sq`, `cosine_score_approx`.
- **stateful / RNG** (struct state): `lcg_next`, `xorshift16`, `bounded_rand`, `counter_*`.
  (`ema_update`/`moving_avg_update` — skip, `q_lerp` already is this: `q_lerp(prev, sample,
  alpha)` is one EMA step.)
- **time / budget** — checked against `docs/cell-index.md` before building: `used_percent` is
  `percent`, `fits_budget` is `is_le`, `cooldown_remaining` is `sub_sat`, `time_until` is
  `sub_sat`, `deadline_missed` is `is_ge` — all aliases, none of these get built as new cells.
- **signed (`i16` now available)**: signed deltas / `lerp` / risk deltas.

## Phase 2.3 — growing toward ~1,000 cells

Wave 3's 20 cells were each authored, then hand-traced against known reference values
(Zeller's congruence checked against 2000-01-01 and 2024-01-01, state-machine transitions
walked by hand) before being written to source. That doesn't scale to ~886 more cells.
The **author → verify → admit** loop below keeps the same rigor but makes the verify step
mechanical instead of hand-traced, so it can run at batch size:

1. **Spec** — one line per candidate: pack, id, intended behaviour, arity hint (free-fn
   ≤3 args vs state cell — remember `u32` can only be a state field, never a free-fn
   call param/return, the constraint the calendrical/checksum pack found). Pull specs from
   this file's "Next waves" list first — already-scoped, not invented fresh.
2. **Author** — draft the cell source + 2-3 retrieval rows (direct/paraphrase) + 2-3
   proposed host-oracle `(args, expected)` triples, using the dialect gotchas already
   learned the hard way this session:
   - `self.field = if ... else ...` **directly** is not supported — bind to a `let` first,
     then assign the field (hit twice: `backoff_next`, `accumulate_step`).
   - `!` is supported logical-not on the 0/1 boolean convention (not bitwise).
   - value-`match` needs a `_` arm; range/or-patterns lower to if-chains.
   - `.saturating_add/sub/mul` work on `u8`/`u16` now (means what it means in host Rust).
3. **Verify — mechanical, not agent-judgment.** This is the step that replaces
   hand-tracing:
   - Compile the candidate; a compile error gets one repair attempt (same shape as the
     `cell-eval repair` eval), else discard.
   - **Actually run** the proposed oracle rows against the real compiled cell and require
     the output to match the claimed expected value — a mismatch means either the cell or
     the claimed expectation is wrong, so it's discarded/flagged either way, never trusted
     on say-so alone.
   - Run `cell80 index --gate` (Phase 2.2) against a scratch copy of the library +
     retrieval dataset with the candidate included — a refusal gets the same treatment as
     every real Wave 3 refusal (alias if it's a true duplicate, discard if not, never
     silently forced through).
4. **Admit** — only candidates passing all three land for real: source file, golden
   regenerated, host-oracle rows, retrieval rows, `docs/cell-index.md` regenerated, a
   `cell-eval curve` checkpoint after the batch (not per-cell) recorded in
   `cell-eval/baselines/library-scale-curve.json`.

**The kill-gate.** Mirroring the escalation ladder's own standing rule (θ calibrated
against a 0.75 precision-on-answered floor, `docs/escalation-ladder.md`): if a checkpoint's
retrieval precision on the paraphrase or adversarial split drops meaningfully from
checkpoint 1's baseline (114 cells: direct 0.94 / paraphrase 0.42 / adversarial 0.39;
`cell-eval/baselines/library-scale-curve.json`), **pause cell growth** and prioritize
discovery/retrieval work instead of adding more cells. A 1,000-cell library nobody can
search is worse than the 114-cell one that can be searched today.

**Known gaps, not yet blocking, worth tracking as the library scales further:**
- The admission gate's fingerprint check only covers arity-≤2 free-fn cells (state cells
  and arity-3 cells are exempt — `cell80/src/admission.rs`'s own doc explains why:
  `Fingerprint`'s probe bank only drives two scalar registers). Generalizing it to typed
  state and arity-3+ is real future work, not done here.
- The admission report isn't wired into CI yet — worth doing whenever CI config is next
  touched, so a duplicate can't land even by accident.
- Keep synthesis (`cell80/src/synth.rs`) gated and narrow, per
  `docs/escalation-ladder.md`'s own "do-not-build" list — it's for outcome-specified tasks
  over a known op family, not a general cell-generation path.

## Mine the ecosystem first

`chuk-math` / `chuk-mcp-math` / `chuk-synthetic-data` likely already hold integer kernels worth
porting straight in — cheaper than authoring from scratch, and it ties the loop.

## After authoring: re-run the evals

Each new **family** is a retrieval test case; each **predicate + transform** pair a composition
test case. Re-run `cell-eval retrieval` / `composition`. Expect direct P@1 to stay strong while
paraphrase stays in coin-flip territory as the library grows (8 → 98 cells: direct ≈ 0.92,
paraphrase 0.53 → 0.45) — that gap is the standing case for the **type-led / capability index** (rank by
typed signature + a `kind = predicate | transform | …` first, embeddings as the tiebreaker). A
big confusable library is precisely what makes that benchmark trustworthy.
