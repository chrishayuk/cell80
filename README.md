# cell80

[![CI](https://github.com/chrishayuk/cell80/actions/workflows/ci.yml/badge.svg)](https://github.com/chrishayuk/cell80/actions/workflows/ci.yml)
&nbsp;License: MIT OR Apache-2.0

**Microsecond-scale, deterministic, sandboxed executable tool capsules for agents.**

A *cell* is a tiny function or state machine — written in a **subset of Rust that is also
real Rust** — compiled to Z80 machine code and run on a **flat-RAM virtual machine**: no OS,
no filesystem, no network, no syscalls, no ambient authority. It can only touch its own
64 KiB of RAM, for a bounded number of cycles, and then it returns a number. That's the
whole sandbox, and you can hold it in your head.

The result is an executable artifact an agent can **discover, inspect, run, compose, and
throw away in microseconds** — tens of bytes of code, cycle-exact and reproducible, with a
typed manifest. The pitch in one line:

> **millions of tiny tools, *retrieved* — not millions of tool schemas dumped into context.**

---

## See it work

Write an ordinary Rust function:

```rust
// score.rs
fn run(x: u16, y: u16) -> u16 { x * x + y * y + x * 3 }
```

Compile and run it on the cell VM — deterministic, sandboxed, headless:

```console
$ cell80 run score.rs --args 10,5 --json
{"abi":2,"entry":"run","result":155,"regs":[155,125,5],"cycles":327,"trapped_ops":2,
 "halt":"returned","code_bytes":47,"functions":1,"memory_touched":[[36864,36867],[65516,65519]]}
```

**47 bytes of code.** It returned `155`, took `327` T-states, touched 7 bytes of RAM, and
halted cleanly — every run reports exactly what it did and what it cost. The *same* `score.rs`
also compiles under `rustc`, so you can debug it as normal Rust; the two are kept honest by
differential testing — every accepted construct is checked against release-mode rustc on
**both** backends (the authentic Spectrum software routines and the cell VM's trap path).
Exactly what an accepted program guarantees — wrapping arithmetic, divide-by-zero policy,
evaluation order, the no-recursion rule — is written down in
[docs/10-dialect-semantics.md](docs/10-dialect-semantics.md).

Freeze it into a self-describing `.cell` cartridge:

```console
$ cell80 compile score.rs -o score.cell --id score.v1 --summary "Score a candidate (x²+y²+3x)" --tags scoring,math
$ cell80 inspect score.cell
cell `score.v1`  (abi 2, compiler 0.8.0)
  Score a candidate (x²+y²+3x)
  tags: scoring, math
  signature: run(x: u16, y: u16) -> u16
  entry: run @ 0x8000
  code: 47 bytes, 1 functions
  capabilities: raw_memory=false ports=false max_code=4096 max_touched=4096
  source_hash: 0xfc466c7b41b2fb31
```

A cartridge carries its own typed interface, tags, capabilities, and a content hash — so a
registry can present it and validate inputs **without re-parsing the source**.

---

## One cell, many bodies (the cell-family)

The cell contract — content-addressed, capability-manifested, cycle-accounted,
differentially verified — is not a Z80 claim. Phase 5
([docs/13-multi-target-spec.md](docs/13-multi-target-spec.md)) extracts the
target-independent core (**`cell80-core`**: the typed IR and its contract, the IR
passes, the reference IR interpreter, the target descriptors) and adds the first
sibling backend: **`rustrv32`**, RV32I(M) for the Hazard3 core in the RP2350 — the
antweight-robot reflex organ this is deployed toward. One source compiles through the
shared IR into independent artifacts that must agree, and the diff battery now runs
every program against **five systems**: release-mode rustc (the oracle), both Z80
targets, the IR interpreter, and the RV32 executor — with the RV32 data window
compared **byte for byte** against the interpreter's memory image.

```console
$ cargo run -p rustz80 --example cell_family
== one source, three bodies: gcd (u16) ==
  Z80/Spectrum48   result= 6021   T-states=12882 (authentic)
  RV32/Hazard3     result= 6021   cycles=490 (provisional table)
  IR interpreter   result= 6021   (the reference meaning)
  agreement: all three identical

== the family widens: deadband (i32) ==
  RV32/Hazard3     result=   11   cycles=301 (signed-32 native)
  IR interpreter   result=   11
  Z80/backend-zero refuses, as pre-registered:
    rustz80: this program uses a signed 32-bit comparison — backend zero doesn't
    emit signed-32 ops … compile it with the RV32 backend (rustrv32, where they
    are native) or run it on the reference interpreter
```

The Z80 T-states are authentic (the cycle-exact core underneath); the RV32 cycle
table is explicitly provisional until `mcycle` on real RP2350 silicon co-signs it
(spec §3, B4). The verification net around the new backend: a seeded
**determinism-fuzz battery** (random width-lattice programs through the whole
matrix, plus the exact-cycle/window fingerprint on fresh executors), and a
**GNU-gas emission adversary** — binutils' independent RISC-V assembler re-encodes
every instruction shape byte-for-byte, required on CI's linux leg, so the encoder
is never self-refereed. The identity story is formal too: `.cell` **v10** manifests
carry a **target id** (which machine body the cartridge holds — a host refuses a
body it can't run) and a **family hash** (SHA-256 over the canonical source, shared
by sibling-target bodies — what "one cell, many bodies" means as an artifact
property, not just a demo). And the first **loadable RV32 cartridge** exists
(WS-E3 slice 1): the cartridge body is per-target (`Body::Z80`/`Body::Rv32` — Z80
serialization unchanged, every existing artifact hash holds), `compile_rv32` runs
the same canonicalization/capability/prelude pipeline through the RV32 backend, and
`Rv32Runner` is the sibling of `Runner` — one source, two cartridges, one family
hash, both bodies running and agreeing, prelude kernels included. Still owed: full
host dispatch by body (`CellHost` over mixed-body libraries), the Sail/spike
*execution* adversary, the RV32 peephole suite, and the silicon co-sign.

The family now reaches the GPU (Phase 6, WS-E): **`rustmsl`** compiles integer
cells — straight-line *and* looping — through the same IR seam to Metal compute
kernels, one thread per input, the interpreter still the one source of meaning.
Loops carry the interpreter's exact fuel discipline, so each thread reports its
**IR-step count** and the batteries assert step parity alongside value parity;
a runaway loop is the same counted trap on both substrates. The gate ran clean
on an M3 Max: **245 value cells × 10⁶ seeded-random inputs, values, status,
and steps bit-identical to the reference interpreter**, at a measured
**3.7×10⁸ evals/s** peak with metering on — and the whole library runs against
a probe set in a single megakernel dispatch (retrieval by execution's
substrate). **State cells run too** — each thread carries a private state
window, driven with adversarial random state bytes and compared on final
state bytes as well: 496/496 bit-exact, taking GPU coverage to **741 of 746
cells** (f32 is next). The batteries earn their keep: they caught a genuine
Apple Metal compiler bug (a divide feeding branch-guarded stores inverts the
branch in non-inlined functions, bisected to a 10-line repro, dodged
structurally in the shipped codegen) *and* two library cells writing through
an unmasked state index — trapped by the GPU's typed window, silently
absorbed by open interpreter memory, filed. A GPU result that disagrees with
the interpreter is a defect, never a "GPU difference".

## The vision

Agents constantly need to *run a little code*: score candidates, validate a move, step a
state machine, check a constraint, compute a reward. Today that means a Python subprocess
(~35 ms just to start, unsandboxed) or a 50 KB Wasm module (fast, but opaque and heavy).
Neither scales to *thousands of tiny tools an agent picks between*.

cell80 makes the unit tiny enough to treat tools like data:

- **Store millions.** A cell is ~tens of bytes of code plus a compact manifest (~2 KB total).
  A million cells is ~2 GB — a file tree or a table, not a fleet of servers.
- **Surface a handful.** The agent `search`es an index of manifests and only ever sees the
  few it's actually considering — context stays small no matter how big the library grows.
- **Run them warm.** A persistent host keeps the index + hot runners in one process, so a
  retrieved cell runs in **~0.05–0.25 µs** — fast enough to call in an inner loop.

```console
$ cell80 search "distance between grid points" cells/
indexed 653 cells; query `distance between grid points` → 10 match(es):
  manhattan — Manhattan distance between two grid points (typed state).  [grid, distance, spatial, score, navigation, taxicab, ...]  (Pts::run() -> u16)
  euclid_sq — Squared Euclidean distance between two grid points: dx*dx + dy*dy (no sqrt). Wide u32 dist field.  [grid, distance, euclidean, squared, spatial, magnitude]  (Pts::run() -> u16)
  manhattan_wide — Manhattan distance between two grid points, into a wide u32 field so it can't silently wrap.  [grid, distance, spatial, wide, u32]  (Pts::run() -> u16)
  chebyshev — Chebyshev (chessboard) distance between two grid points: max(|dx|, |dy|).  [grid, distance, chebyshev, chessboard, spatial]  (Pts::run() -> u16)
  manhattan_i16 — Manhattan distance with signed (i16) coordinates, into a wide u32 field.  [grid, distance, spatial, signed, i16, wide, u32]  (PtsSigned::run() -> u16)
  ...                                                                   (euclid_sq_i16 and 5 more, lower-ranked)
```

The loop an agent runs: **`search` → `inspect` → `run` → discard** — over a library that may
hold far more cells than belong in any context window. A good cell is one whose *manifest is
smaller than its usefulness*.

> **cell80 is not a faster Wasm — it's a manifest-addressable executable micro-tool format
> for agents.** A `.cell` is closer to an *executable index card* than a plugin: a tiny
> deterministic behaviour with a typed signature, a hash, a cost surface, a capability
> policy, and bounded execution. A tool shouldn't need a server, a process, or a page of
> schema if it's only 47 bytes of behaviour.

### Does it work? Measure it — find → run → compose.

The pitch only matters if an agent *actually* uses cells instead of writing the code itself.
[`cell-eval`](./cell-eval) measures the whole arc, with **steering held fixed** so index/library
changes and prompt changes never get conflated:

- **retrieval precision** — deterministic, no model: given a query, is the right cell in the
  top-k? (Reads index quality directly.)
- **adoption** — an LLM agent over an **OpenAI-compatible / Ollama** endpoint: given a task, did
  it `search → inspect → run` the right cell, and get the right answer?
- **composition** — given a task that needs *several* cells, did it **wire them together** (via
  `cell_graph_run`) instead of doing the multi-step arithmetic itself?

Retrieval **on the 395-cell library** (checkpoint 17, 797 cases; the curve now runs
through checkpoint 21 at 653 cells — `docs/library-growth.md` carries the full history,
including the round-3 kill-gate trip and its resolution): the
default index is
**TF-IDF** (word + char-3-gram cosine), with a small complexity-based tie-break — **direct
P@1 0.82, paraphrase 0.39, adversarial 0.42**. Against the 114-cell baseline the library
has grown 3.5× with paraphrase essentially flat and adversarial still *above* it (+2.3
pts) — the registered kill-gate does not trip — but adversarial fell ~8 pts from the
209-cell checkpoint as the library filled with deliberately similar family members
(`min`/`min_u32`, width/signedness siblings). That residual is *same-shape siblings*
(`min`/`min_u32`,
the sign-magnitude family `smag_add`/`smag_sub`/`smag_mul`/`smag_div`/`smag_cmp`) that a
predicate/transformer signal can't separate, since both sides of each pair are
non-predicates. The lever for those is **behavioural I/O-example routing**
(`cell_route_by_example`): on `(3,7)→3` only `min` matches, not `max` — selection grounded in
what the cell *does*, phrasing- and language-independent. Now also a top-level CLI verb
(`cell80 route`, see Quick start), and with `--facts` the probes are answered from an imported
fact file (docs/12) instead of execution — retrieval served from claims, falsifiable by re-running.
**As of 2026-07-11 that lever is fused into the primary search path** (WS-F/F2, measured at
653 cells): `search` takes optional I/O examples — behaviour ranks, text order breaks ties,
zero-match cells demote instead of dropping, so the fused rank is provably never worse than
plain search. Example-carrying paraphrase queries go **0.39 → 0.859 P@1** (adversarial
0.47 → 0.89, direct 0.81 → 0.95; `cell-eval retrieval --examples retrieval-examples`,
checkpoint 21 in `cell-eval/baselines/`, hard CI floor). Text-only paraphrase is unchanged —
still the open problem for text-side levers.
Beyond text search: **`cell80 solve`** compiles a candidate math word-problem plan directly
to a cell and verifies/perturbs it (`docs/math-campaign-spec.md`'s M2) — 127 real, unfiltered,
consecutive GSM-8K test-set rows hand-extracted and run through it solved **123/123 correctly**
(4 skipped as genuinely unrepresentable, ~97% representability;
`cargo run --release --example m3_gsm8k_smoketest -p cell80`), the first real evidence
beyond hand-crafted cases that the loop works end to end.
Adoption/composition (`granite4.1:3b`): **adoption 1.00 / correct 1.00**; composition once read
**composed 0.50 / correct 0.83 — but `used_graph` 0.00**: the small model *chains* cell calls
and never authors the wire-level graph manifest. That finding drove a fix — **`cell_compose`**,
a pipeline-authoring helper (an ordered list of `{cell, args}` with positional args, ports
resolved from the manifest; no wires). With it the same model **composes via a pipeline in
half the tasks** (`used_pipeline` 0.50, raw `used_graph` still 0.00), and **composed 0.79 /
correct 0.93** — graph-authoring ergonomics, not the VM, was the lever.

---

## Quick start

```bash
git clone https://github.com/chrishayuk/cell80 && cd cell80

# run a cell from source (cells live in pack subdirectories: cell80/cells/<pack>/<id>.rs)
cargo run -p cell80 --bin cell80 -- run cell80/cells/number-theory/gcd.rs --args 1071,462
# → result 21

# browse + search the seed library, then run one warm in a persistent session
cargo run -p cell80 --bin cell80 -- index  cell80/cells
cargo run -p cell80 --bin cell80 -- search "validate a value in range" cell80/cells
printf 'load gcd\nrun 0 1071,462\nquit\n' | cargo run -q -p cell80 --bin cell80 -- serve cell80/cells

# don't know the name? route by BEHAVIOUR — which cells reproduce these input→output examples?
cargo run -q -p cell80 --bin cell80 -- route cell80/cells 3,7=3 10,4=4 255,1=1
# → min — Minimum of two values.  [3/3]   (flip the outputs and `max` wins instead;
#   `median3` ties min at [3/3] — with the third register 0, median3(a,b,0) IS min(a,b))

# or FUSE both: search with trailing examples — behaviour ranks, text breaks ties.
# Same-shape siblings share every word; the examples separate them (WS-F, checkpoint 21).
cargo run -q -p cell80 --bin cell80 -- search "the smaller of two numbers" cell80/cells 3,7=3 9,4=4
# state cells route by named field; `out:12` matches POST-RUN fields — the separator for
# status-flag families (smag_add/smag_sub both return 1; only their mag/neg differ)
cargo run -q -p cell80 --bin cell80 -- search "combine two magnitudes" cell80/cells mag_a:9,neg_a:0,mag_b:4,neg_b:1=1,mag:5,neg:0

# the same route riding the FACT LIBRARY (docs/12): imported claims answer probes, no execution
printf 'min 3 7\nmin 10 4\nmin 255 1\n' > /tmp/calls.txt
cargo run -q -p cell80 --bin cell80 -- facts export cell80/cells --calls /tmp/calls.txt > /tmp/min.facts
cargo run -q -p cell80 --bin cell80 -- route cell80/cells 3,7=3 10,4=4 255,1=1 --facts /tmp/min.facts
# → probe runs: 489 — 3 answered from imported facts, 486 computed locally
```

CLI verbs: `run` (source) · `compile` (→ `.cell`) · `exec` (a `.cell`) · `inspect` ·
`index` · `search` (text relevance; trailing I/O examples fuse behaviour into the ranking) ·
`route` (by I/O examples, optionally answered from a fact file) ·
`serve` (a persistent warm session) · `facts` (export / import / verify — the full contract
lives in [`docs/12-fact-file.md`](./docs/12-fact-file.md): a fact is keyed by the artifact
hash — which covers the image *and* its capability/halt policy — carries its cycle cost, and
is **verified by re-execution**, never trusted; a changed cell is a different hash, so facts
cannot go stale, only unresolvable) · `solve` (candidate plans compile to cells and get
verified/perturbed — `docs/math-campaign-spec.md` M2, demo: `cargo run --release --example
solve_demo -p cell80`).
Same commands, drivable by an agent over JSON.

---

## Sandboxing, security & determinism

A cell has **no ambient authority** — the VM has no ROM, no I/O ports, no syscalls, no host
calls except the arithmetic traps. By construction it can do nothing but compute over its
own RAM. On top of that:

- **Capability-gated, sandboxed by default.** Raw memory (`poke`/`peek`) and I/O ports are
  *off* unless explicitly granted; `max_code_bytes` and `max_touched` cap size and footprint.
  The CLI runs sandboxed unless you opt in (`--allow-raw-memory`, `--max-touched N`, …).
- **Bounded.** Every run has a cycle budget; an infinite loop stops and reports
  `halt: cycle_budget` instead of hanging. A run always tells you *why* it stopped
  (`returned` / `halted` / `cycle_budget` / `memory_limit` / `div_by_zero`).
- **No garbage flows onward.** `/ 0` halts the run (`halt: div_by_zero`) instead of
  yielding a saturated quotient into downstream scoring, and **recursion is rejected at
  compile time** (static locals make it silently wrong — so it doesn't compile).
- **Deterministic.** Same inputs → same result, same cycle count, same touched-memory set —
  fuzzed across rerun, fresh instance, image round-trip, and the fast vs. authentic
  executor. Reproducible by design (no clocks, no RNG, no I/O).
- **Honest about cost.** Reports carry `cycles` *and* `trapped_ops` — host traps (mul/div)
  are near-free in T-states, so they're counted separately, which means a reward function
  can't be gamed by routing work through them.
- **Conformance-tested core.** The Z80 the cells run on passes the per-opcode
  **SingleStepTests** suite — **1,530,000 / 1,530,000** cases across the full instruction set
  (base/CB/ED/DD/FD/DDCB/FDCB), including cycle counts and the undocumented flags — plus the
  **ZEXDOC** exerciser ROM. So "cycle-exact" is *measured*, not asserted ([`z80-tests/`](./z80-tests)).

The whole trust surface is *64 KiB of RAM + a cycle budget* — small enough to audit, and
the same for every cell.

### Non-goals — the moat is what it refuses to be

- **Strings, floats-by-default, I/O, network.** A cell that needs them isn't a cell —
  that's the escalation path to the host (a typed hand-off), not a roadmap item. The
  moment the ISA chases general applicability, the differentiation vs Wasm evaporates.
- **JIT / speed chasing.** Wasm wins warm compute; the moat here is exact metering,
  auditability, byte-scale artifacts, and determinism. Protect the moat — don't race
  the loser's race.

---

## Benchmarks

`cell-bench` compares running a tiny agent-shaped program — score `N` candidates with
`x*x + y*y + 3*x` — across four runtimes (Apple Silicon; `cargo run --release --manifest-path cell-bench/Cargo.toml`):

```
runtime            per-call   cold setup    batch(1000)   result-sum
--------------------------------------------------------------------------
native Rust        0.001 µs            —       0.681 µs   2722460
wasmtime           0.013 µs  2876.000 µs      12.623 µs   2722460
cell (report)      0.499 µs   540.667 µs     499.391 µs   2722460
cell (fast)        0.237 µs            —      237.346 µs   2722460
python (subp)     36.911 µs 34892.000 µs  36911.083 µs   2722460
```

**This is not a claim that the cell is the fastest compute** — a Wasm JIT wins warm compute
by ~18×, exactly as it should; for a heavy algorithm, use Wasm. What the cell wins for the
*tiny-tool* class:

- **~5× lower cold setup** than Wasmtime (0.59 ms vs 3.0 ms) — and for a *cached* snippet,
  re-instantiation is **~1.2 µs** (~2500× under Wasm's JIT), because setup is ~90% `syn`
  parsing and a cached `CellProgram` skips it.
- **Code ~1070× smaller** — 47 bytes of Z80 vs a ~50 KB Wasm module. Small enough to
  inspect, hash, cache, or show a human.
- **Far lighter than a Python subprocess** (~37 µs/call amortized, ~35 ms to start).
- Plus what a table can't show: determinism, typed state read-back, capability gating, and a
  sandbox you can hold in your head.

The defensible sweet spot is **cold, long-tail retrieval** — instantiating a cell you've
never seen, where setup dominates and the cell is cheapest (and which is exactly the
retrieve-a-tool-you-rarely-use case). For a *hot* inner loop over a small fixed set, Wasm's
warm advantage reasserts — at ~0.24 µs/call the cell still clears ~4M evals/s, but that's not
the regime to choose it for. Pick the cell when the tool is **disposable and rarely-seen**,
not when it's a hot kernel you'd keep resident.

---

## Write a cell

The dialect is a bounded subset of real Rust — `u8`/`u16`/`u32`/`i16`, arithmetic (incl. `if`/`match` as values), comparisons as
values (`(a < b) as u16`) + `&&`/`||`, runtime bit shifts, `if`/`while`/`for`/`loop`, arrays,
`struct`/`enum`/`match` (incl. nested struct fields — `self.sprite.x`), functions and methods, generics (monomorphized), top-level `const`
items (scalar substitution + a by-address const-data section — tiles, tables, and string
literals as interned length-prefixed bytes), `poke`/`peek`. A few of the cells
(`cell80/cells/`):

```rust
// gcd — Euclid's algorithm (a loop; div/mod are host traps in cell mode)
fn run(a: u16, b: u16) -> u16 {
    let mut x = a;
    let mut y = b;
    while y != 0u16 { let t = x % y; x = y; y = t; }
    x
}
```

```rust
// manhattan — typed state in, typed state out (no raw addresses)
struct Pts { x1: u16, y1: u16, x2: u16, y2: u16, dist: u16 }
impl Pts {
    fn run(&mut self) -> u16 {
        let mut dx = 0u16;
        if self.x1 > self.x2 { dx = self.x1 - self.x2; } else { dx = self.x2 - self.x1; }
        let mut dy = 0u16;
        if self.y1 > self.y2 { dy = self.y1 - self.y2; } else { dy = self.y2 - self.y1; }
        self.dist = dx + dy;
        self.dist
    }
}
```

For a `&mut self` method the host sets named fields, runs, and reads named fields back — the
JSON↔state surface an agent (or the MCP server) drives, with no raw addresses. Anything
outside the subset is a **clear compile error** — that error is the "this belongs in host
code, not a cell" signal. The full language reference is in
[`rustz80/README.md`](./rustz80/README.md) and [`docs/07`](./docs/07-rust-z80-compiler-spec.md).

Cells stay **modular**, not copy-pasted: a small shared **kernel prelude** (`gcd`, `imin`,
`imax`, `iabs_diff`, `isqrt`, `clamp_to`) is appended to every cell, so `lcm` just calls `gcd`
and `chebyshev` calls `iabs_diff`/`imax`. **Dead-code elimination** then drops the kernels a
cell doesn't use, so a cartridge only ever carries what it reaches — a cell that touches no
kernel is byte-identical to having no prelude at all.

**The envelope is deliberately narrow** — integers only:

| type | width | notes |
|---|---|---|
| `u8` | 1 slot | zero-extends on load; wrapping |
| `u16` | 1 slot | the native word; wrapping |
| `i16` | 1 slot | two's complement: signed compare/divide/`>>`; wrapping |
| `u32` | 2 slots | wide arithmetic + state fields; wrapping |
| fractional | — | a **fixed-point convention on integers** (Q8.8: `(a * w) >> 8`), not a float type |

no floats,
fixed-size structs/arrays, 64 KiB, no string *type*/syscalls (string **literals** compile
as addressable const data — length-prefixed bytes, not a `String`). That's a real class: a tiny
deterministic integer **stdlib** (predicates, percentages, bounds, ranking, bit/flag ops —
the [first wave](./docs/library-growth.md)), scoring, validators, range/move checks, small
state machines, reducers, grid logic, RNGs, reward kernels. It is *not* "any tool an agent wants" (most of those want a float, a string, or a
syscall — all compile errors here, by design). So the sharpest **near-term beachhead is
deterministic, bounded, cycle-honest kernels** — move-validation and reward computation for
rate-decoupled RL (e.g. SOMA) — where the integer envelope fits exactly and determinism is
the whole point. The broad "tool substrate" vision is the direction; reward/validation
kernels are the wedge.

---

## Connect it to an LLM (MCP)

`cell80-mcp` exposes a warm library over MCP as a thin **router** — not a tool per cell.
A few fixed tools (`cell_search` / `cell_route_by_example` / `cell_inspect` / `cell_list` /
`cell_run` / `cell_compose` / `cell_graph_run`) let a model find, run, and *compose* the few
cells it wants while the library stays out of context. Built on the PyO3 binding `cell80-py`
(the warm host as a Python class), the same Rust-core → PyO3 → Python-MCP shape as the rest of
the ecosystem.

```python
cell_search("grid distance")     # → a few brief manifests
# don't know the name, or the words are ambiguous? route by BEHAVIOUR — the cell that
# reproduces these input→output examples (tells `min` from `max` where text can't):
cell_route_by_example([{"in": [3, 7], "out": 3}, {"in": [10, 3], "out": 3}])  # → min, not max
# or fuse both in one call: examples rank first, text relevance breaks ties (WS-F):
cell_search("the smaller of two numbers", examples=[{"in": [3, 7], "out": 3}])
cell_inspect("manhattan")        # → Pts::run() -> u16, typed state
cell_run("gcd", [1071, 462])     # → {result: 21, cycles, trapped_ops, halt}  (warm)
# state cells drive by NAME — typed fields in, full state out (no raw addresses):
cell_run("manhattan", fields={"x1": 3, "y1": 4, "x2": 10, "y2": 8})
#   → {result: 11, state: {x1: 3, y1: 4, x2: 10, y2: 8, dist: 11}, cycles, …}
# COMPOSE the easy way: a PIPELINE — positional args ("$N" = step N's result), ports resolved
# from the manifests. No wires, no port names. (cell_graph_run takes the full manifest for DAGs.)
cell_compose(
    steps=[{"cell": "manhattan",    "args": ["x1", "y1", "x2", "y2"]},
           {"cell": "weighted_sum", "args": ["$0", "risk", "cost"]},
           {"cell": "clamp",        "args": ["$1", 0, 10]}],
    inputs={"x1": 3, "y1": 4, "x2": 10, "y2": 8, "risk": 2, "cost": 1})
#   → {outputs: {out: 10}, trace: [s0→11, s1→18, s2→10], cycles, …}
```

---

## The crates

| crate | what it is |
|-------|------------|
| **[`cell80-z80`](./z80)** | a cycle-accurate Z80 CPU core (`no_std`-friendly, dependency-free). Import name `z80`. |
| **[`cell80-core`](./cell80-core)** | the **target-independent compiler core** (Phase 5): the typed IR and its semantic contract, the inline/DCE passes, the reference IR interpreter, and the target descriptors. Dependency-free. |
| **[`rustz80`](./rustz80)** | the restricted-Rust → Z80 compiler (**backend zero**): `syn` frontend → `cell80-core` IR → Z80 codegen. Differential-tested against `rustc`. |
| **[`rustrv32`](./rustrv32)** | the **RV32I(M) sibling backend** (Phase 5, WS-B): its own symbolic `Ins` layer + exact encoder, a cycle-accounted RV32IM executor (Hazard3/RP2350-shaped), and codegen over the shared IR — live in the same diff battery. |
| **[`rustmsl`](./rustmsl)** | the **Metal (MSL) sibling backend** (Phase 6, WS-E): IR→MSL codegen for integer cells (loops included, fuel-metered tick-for-tick) + a batch GPU executor and the library×probes megakernel — bit-exact against the reference interpreter on values *and* IR steps (245 cells × 10⁶ inputs; 3.7×10⁸ evals/s measured). |
| **[`cell80`](./cell80)** | the **cell micro-VM + tooling** (built on `rustz80`): `.cell` cartridges, a compile-once/run-many `Runner` + `CellPool`, a decode-once fast path, `CellIndex`, the warm `CellHost`, typed-state I/O, host-routed `CellGraph` composition, and the `cell80` CLI. |
| **[`cell80-py`](./cell80-py)** | PyO3 bindings — the warm `CellHost` as a Python class (built with maturin). |
| **[`cell80-mcp`](./cell80-mcp)** | the MCP server over a warm cell library (`chuk-mcp-server`). |
| **[`cell-eval`](./cell-eval)** | the agent eval harness — retrieval precision + LLM adoption + composition (does an agent find, run, and *compose* the right cells instead of writing code?). |
| **[`cell-bench`](./cell-bench)** | the cross-runtime comparison (native / Wasmtime / cell / Python). |
| **[`z80-tests`](./z80-tests)** | the Z80 conformance harness — SingleStepTests vectors + ZEXDOC. |

The roadmap (`docs/roadmap.md`) tracks the agent eval harness, typed-state I/O over MCP
(done), the **standard library** (done — **653 cells** across 30+ families incl. the
math-campaign packs: checked/exact wide arithmetic, fractions, money/bps, units,
verifier/ranker, sign-magnitude — plus the compiler
ergonomics that make predicates/bitops one-liners and a **shared-kernel prelude + dead-code
elimination** so cells reuse `gcd`/`imin`/`iabs_diff`/… instead of re-implementing them),
**host-routed `CellGraph` composition** (cells wired into a static, type-checked graph the host
validates before running) with the **`cell_compose`** pipeline helper that lets a small model
actually author one (measured: composition `used_graph` 0.00 → `used_pipeline` 0.50), and
**`cell_solve`** (M2 of the math campaign, `docs/math-campaign-spec.md`): a candidate plan
renders straight to a cell, compiles, runs, and gets perturbed — schemas precipitate by
artifact hash (a re-seen structure is *retrieved*, not recompiled), and the counterfactual
battery always stress-tests survivor agreement, not just disagreement. A 25-problem,
unfiltered slice of the real GSM-8K test set solved 25/25 through it.
The execution plan is phased in [`docs/roadmap-phases.md`](docs/roadmap-phases.md):
**Phase 0 (shipped)** closed the determinism contract — recursion rejected at compile time,
the Cell trap path differential-tested against rustc, `/ 0` a typed halt, the dialect
semantics written down — then the LLM-facing compiler (if/match expressions, diagnostics,
signed `i16` — shipped), retrieval as the product, trust (signed cells, escalation contract,
memoization), and codegen stage 2 (the symbolic `Ins` layer + the measured peephole —
shipped, −4.3 % corpus code size, plus a later `INC` strength-reduction rule at −497 B more;
u32 array elements remain).
**Phase 5 (in flight)** opens the **multi-target cell-family**
([docs/13-multi-target-spec.md](docs/13-multi-target-spec.md)): WS-A complete
(target descriptor, reference IR interpreter, canonical evaluation order, the
explicit width-bridge family with `SignExtend`, i32 through the IR, the
`cell80-core` extraction); WS-B's compiler live and fenced (B1–B3 plus the
determinism-fuzz battery and the CI-required GNU-gas emission adversary; byte-exact
window parity with the interpreter; per-file coverage ≥90%); WS-E underway
(`.cell` v10 target id + family hash; the state-layout question resolved by the
family slot ABI; the first loadable RV32 cartridge via `compile_rv32` +
`Rv32Runner`). **Phase 6 is open**
([docs/14-model-native-cells-spec.md](docs/14-model-native-cells-spec.md) —
model-native cells: GPU bodies over the same IR seam, retrieval by execution,
decode-time wiring, trained invocation): the F2 routing gate passed
(probe-equipped paraphrase P@1 0.859 vs the 0.80 bar) and **E1–E3 landed on
Metal** — `rustmsl` compiles the library's integer value cells (loops
included, fuel-metered tick-for-tick) to Metal compute kernels; the
pre-registered gate ran clean (245 cells × 10⁶ random inputs, values +
status + IR steps bit-exact) at a measured 3.7×10⁸ evals/s, with the whole
library runnable against a probe set in one megakernel dispatch.
Owed from Phase 5: full `CellHost` dispatch by body,
the Sail/spike execution adversary, the RV32 peephole suite, and the RP2350
`mcycle` co-sign (B4).

---

## Origin

cell80 began as the compute-cell layer of
[`chuk-speccy`](https://github.com/chrishayuk/chuk-speccy) (a ZX Spectrum emulator + agent
platform) and was extracted, with full history, once the `.cell` artifact, CLI, and MCP
adapter made it a product in its own right. `chuk-speccy` now depends on cell80 for its Z80
core and compiled game logic; cell80 still emits authentic Z80/Spectrum output where needed.

## License

Dual-licensed under either [MIT](./LICENSE-MIT) or [Apache-2.0](./LICENSE-APACHE), at your
option.
