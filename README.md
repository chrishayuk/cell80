# cell80

[![CI](https://github.com/chrishayuk/cell80/actions/workflows/ci.yml/badge.svg)](https://github.com/chrishayuk/cell80/actions/workflows/ci.yml)
&nbsp;License: MIT OR Apache-2.0

**Microsecond-scale, deterministic, verified executable tool capsules for agents and models.**

A *cell* is a tiny function or state machine — written in a **subset of Rust that is also
real Rust** — compiled through a shared typed IR and run under one contract everywhere:
no OS, no filesystem, no network, no syscalls, no ambient authority. A cell touches only
its own memory, for a bounded number of steps, and then it returns a typed result. That's
the whole sandbox, and you can hold it in your head.

The central claim is **verified execution over predicted execution** — and it is not a
claim about any particular chip. The same source cell runs on:

- a **cycle-accurate Z80 micro-VM** (backend zero — where the project started),
- **RV32I on real silicon** (Hazard3 / RP2350 — a robot's reflex organ),
- the **GPU** (Metal today; one thread per (cell, input), whole-library megakernels),

with the **reference IR interpreter as the single oracle**: a result on any body that
disagrees with the interpreter — value, trap status, step count, or state byte — is a
defect, never a "target difference".

The pitch in one line:

> **millions of tiny verified tools, *retrieved* — not millions of tool schemas dumped into context.**

And the reason the GPU body exists, from the model-native spec:

> **the model provides judgment; the cells provide guarantees; the interface between them is fast enough to sit inside the model's own thought.**

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
halted cleanly — every run reports exactly what it did and what it cost. The *same*
`score.rs` also compiles under `rustc`, so you can debug it as normal Rust; the two are
kept honest by differential testing — every accepted construct is checked against
release-mode rustc on both Z80 backends. Exactly what an accepted program guarantees —
wrapping arithmetic, divide-by-zero policy, evaluation order, the no-recursion rule — is
written down in [docs/10-dialect-semantics.md](docs/10-dialect-semantics.md).

Now run cells by the million on the GPU (macOS):

```console
$ cargo run --release -p cell80 --example gpu_cells
== one source, one more body: deadband (u16) ==
  deadband(498,500,10)  GPU=500 steps=20  interpreter=500 steps=20  agree (values AND IR steps)
== the whole library × a probe set, one megakernel dispatch ==
  249 cells × 16 probes = 3984 evals in one launch — every cell's behaviour at once
== a state cell stepping on the GPU (typed state, chained) ==
  ok=0 → st=1 state=[1, 0, 3, 0, 4, 0]  fail — trips           (interpreter agrees)
```

Four acts against one rule: the interpreter is the one source of meaning. A cell runs as
a Metal compute kernel with **bit-exact agreement on values, trap status, IR-step counts,
and state bytes**; a batch layout evaluates one cell across a million inputs in a single
dispatch; the whole eligible library fuses into a megakernel and runs against a probe set
in one launch; a state cell steps a state machine across dispatches, its typed state
window chained through unified memory.

Freeze any cell into a self-describing `.cell` cartridge:

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

A cartridge carries its own typed interface, tags, capabilities, and a content hash — so
a registry can present it and validate inputs **without re-parsing the source**.

---

## One IR, many bodies

Everything shared between backends lives in **`cell80-core`**, with nothing of any
backend inside: the typed IR with explicit widths, the family-wide slot ABI, explicit
width bridges, pinned left-to-right evaluation order, the IR-to-IR passes, and the
**reference interpreter — the one executable definition of IR semantics**. The canonical
abstract cost of a cell is **IR steps as counted by the interpreter**; Z80 T-states, RV32
cycles, and GPU wall-time are per-target refinements recorded in each target's
descriptor. Cost is target-honest by construction.

| body | role | status |
|---|---|---|
| **Z80** (`rustz80` + `cell80-z80`) | backend zero — the warm micro-VM, the CLI, the library host; still emits authentic Spectrum output where needed | shipped, conformance-clean (1.53M SingleStepTests vectors, ZEXDOC) |
| **RV32I(M)** (`rustrv32`) | real silicon — Hazard3 on RP2350, the antweight robot's reflex organ; encoder refereed by a GNU-gas emission adversary | shipped (Sail/spike execution referee + `mcycle` co-sign still owed) |
| **Metal / MSL** (`rustmsl`) | the GPU body — batch evaluation, reward organs, whole-library megakernel dispatch, per-thread typed state | shipped for the integer library, value + state cells, bit-exact incl. IR steps and state bytes (f32/E4 owed; two filed cell defects excluded) |
| **CUDA** | serving + training hardware; batch rewards and device residency | owed — sequenced immediately before trained invocation needs it |
| **WGSL** | portable body; the browser demo | owed |

Every backend answers to the same discipline: differential testing against `rustc` on the
source side, bit-exact parity with the interpreter on the execution side, and per-ISA
instruction layers that share *discipline*, not code. The batteries have already earned
their keep as a semantic audit: they caught a **real Apple Metal compiler bug** (a divide
feeding branch-guarded stores compiles with the branch inverted in non-inlined functions
— bisected to a 10-line repro, dodged structurally, so the shipped GPU configuration is
exactly the battery-validated one), and **two library cells writing through an unmasked
state index** — trapped by the GPU's typed window, silently absorbed by open interpreter
memory, filed.

---

## The library

The seed library has grown to **782 cells across 42 packs** — math, statistics,
Excel-shaped financial functions, an owned f32 softfloat surface (bit-identical to
rustc), calendars and checksums, sliding-window and state-machine families — each behind
an **admission gate with behavioural fingerprints** that catches the duplicates
per-candidate verification misses.

The loop an agent runs: **`search` → `inspect` → `run` → discard** — over a library that
may hold far more cells than belong in any context window. A good cell is one whose
*manifest is smaller than its usefulness*.

```console
$ cell80 search "distance between grid points" cells/
  manhattan — Manhattan distance between two grid points (typed state).  [grid, distance, spatial, ...]  (Pts::run() -> u16)
  euclid_sq — Squared Euclidean distance between two grid points: dx*dx + dy*dy (no sqrt).  [grid, distance, ...]  (Pts::run() -> u16)
```

Whether an agent *actually* retrieves and runs the right cell is the thesis — and it is
measured, not assumed. **`cell-eval`** scores retrieval precision, LLM adoption, and
composition as separate numbers that fail for different reasons. The headline retrieval
result (WS-F, checkpoint 21 at 653 cells): text-only paraphrase P@1 is **0.39**; adding
behavioural I/O examples to the same queries — *search by what the cell does, ranked by
running the candidates* — lifts it to **0.859** (adversarial 0.47 → 0.89, direct → 0.95).
Behaviour ranks, text breaks ties, and a fused query is provably never worse than plain
search. The GPU megakernel is that idea's hardware substrate: the whole library against a
probe set in one dispatch.

---

## Sandboxing, security & determinism

A cell has **no ambient authority** — no ROM, no I/O, no syscalls, no host calls except
the arithmetic traps. By construction it can do nothing but compute over its own memory.
On top of that:

- **Capability-gated, sandboxed by default.** Raw memory and I/O ports are *off* unless
  explicitly granted; `max_code_bytes` and `max_touched` cap size and footprint. The CLI
  runs sandboxed unless you opt in.
- **Bounded.** Every run has a budget; a runaway loop is a counted trap — `halt:
  cycle_budget` on the VM, a per-thread fuel trap on the GPU — never a hang. A run always
  tells you *why* it stopped.
- **Deterministic.** Same inputs → same result, same step count, same touched-memory
  set — fuzzed across rerun, fresh instance, image round-trip, executor variants, and
  now across bodies. No clocks, no RNG, no I/O; the f32 surface is owned softfloat, so
  there is no float nondeterminism to tolerate anywhere.
- **Honest about cost.** Reports carry steps *and* `trapped_ops` — host traps are
  near-free, so they're counted separately, which means a reward function can't be gamed
  by routing work through them.
- **Verified once, remembered forever.** Because runs are deterministic, every
  bit-exactness verdict memoizes as a content-addressed **oracle transcript**
  (docs/12's fact-file idea): re-verifying the whole library against the GPU costs a
  digest compare, not an interpreter run.

The whole trust surface is *a bounded memory window + a step budget*, identical in shape
on every body — small enough to audit, and the same for every cell.

---

## Measured numbers

All figures measured, none extrapolated (Apple Silicon unless noted):

- **~0.05–0.25 µs/call** warm CPU fast path — millions of evaluations/second inside an
  agent loop
- **~1.2 µs** re-instantiation of a cached cell (~2500× under a Wasm JIT's cold path);
  ~5× lower cold setup than Wasmtime from source
- **47 bytes** of code for the demo cell — ~1000× smaller than an equivalent Wasm module;
  small enough to inspect, hash, cache, or show a human
- **3.7×10⁸ evals/s** one-cell GPU peak (M3 Max, end-to-end, fuel metering on)
- **The full integer library bit-exact on the GPU** across values, status, IR steps, and
  state bytes — value and state cells both; the remainder is the f32 bank (owed) plus two
  filed defects
- **Whole-library megakernel launch: ~140–180 ms flat** from 8 to 512 probes — measured
  and recorded as the owed optimization (split pipelines / function tables) that the
  retrieval story depends on; not hidden inside the peak number above
- The pre-registered 10⁶-inputs-per-cell oracle gate cost ~4×10¹² interpreter ticks when
  first run; profiling by GPU step counts found seven cells carrying 99.9% of it, and
  value-identical rewrites (audited old-vs-new on the GPU) cut the bill ~7× — with
  transcripts, re-running the gate now costs seconds

`cell-bench` holds the cross-runtime comparison (native / Wasmtime / cell / Python
subprocess); the honest summary is that a Wasm JIT wins warm heavy compute by ~18×,
exactly as it should — the cell wins the *tiny-tool* class on cold cost, artifact size,
determinism, and a sandbox you can audit.

---

## Connect it to an LLM (MCP)

`cell80-mcp` exposes a warm library over MCP as a thin **router** — not a tool per cell.
A few fixed tools (`cell_search` / `cell_inspect` / `cell_list` / `cell_run`) let a model
find and run the few cells it wants while the library stays out of context. Built on the
PyO3 binding `cell80-py` (the warm host as a Python class). `cell_search` takes optional
I/O examples — the fused behavioural ranking over MCP.

```python
cell_search("grid distance")     # → a few brief manifests
cell_inspect("manhattan")        # → Pts::run() -> u16, typed state
cell_run("gcd", [1071, 462])     # → {result: 21, cycles, trapped_ops, halt}  (warm)
```

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
# → min — Minimum of two values.  [3/3]   (flip the outputs and `max` wins instead)

# or FUSE both: search with trailing examples — behaviour ranks, text breaks ties.
cargo run -q -p cell80 --bin cell80 -- search "the smaller of two numbers" cell80/cells 3,7=3 9,4=4

# state cells route by named field; `out:` entries match POST-RUN fields
cargo run -q -p cell80 --bin cell80 -- search "combine two magnitudes" cell80/cells mag_a:9,neg_a:0,mag_b:4,neg_b:1=1,mag:5,neg:0

# the same route riding the FACT LIBRARY (docs/12): imported claims answer probes, no execution
printf 'min 3 7\nmin 10 4\nmin 255 1\n' > /tmp/calls.txt
cargo run -q -p cell80 --bin cell80 -- facts export cell80/cells --calls /tmp/calls.txt > /tmp/min.facts
cargo run -q -p cell80 --bin cell80 -- route cell80/cells 3,7=3 10,4=4 255,1=1 --facts /tmp/min.facts

# the GPU demo (macOS)
cargo run --release -p cell80 --example gpu_cells
```

CLI verbs: `run` · `compile` (→ `.cell`) · `exec` · `inspect` · `index` · `search` ·
`route` · `solve` · `facts` · `serve` (a persistent warm session). Same commands,
drivable by an agent over JSON.

---

## The crates

| crate | what it is |
|---|---|
| **[`cell80-core`](cell80-core)** | the target-independent core: typed IR, slot ABI, IR passes, target descriptors, and the **reference interpreter** — the one executable definition of cell semantics |
| **[`rustz80`](rustz80)** | restricted-Rust → Z80: `syn` frontend → typed IR → Z80 codegen, differential-tested against `rustc` |
| **[`rustrv32`](rustrv32)** | the RV32I(M) sibling backend — Hazard3/RP2350 deployment, gas-adversary-refereed encoder |
| **[`rustmsl`](rustmsl)** | the Metal backend — IR → MSL codegen (builds everywhere), batch GPU executor + megakernel (macOS) |
| **[`cell80`](cell80)** | the cell layer: `.cell` cartridges, admission + fingerprints, the warm `CellHost`, `CellIndex`, the CLI |
| **[`cell80-z80`](z80)** | the cycle-accurate Z80 CPU core (`no_std`-friendly, dependency-free) |
| **[`cell80-py`](cell80-py)** | PyO3 bindings — the warm host as a Python class |
| **[`cell80-mcp`](cell80-mcp)** | the MCP server over a warm cell library |
| **[`cell-bench`](cell-bench)** | the cross-runtime comparison |
| **[`cell-eval`](cell-eval)** | the agent eval harness — does an agent find → run → compose the right cell? |
| **[`experiments/`](experiments)** | pre-registered experiments with findings docs (evolution, GoL, small-model pilots) |

---

## Where it's going

The specs are the roadmap, each with pre-registered gates
([docs/roadmap-phases.md](docs/roadmap-phases.md) sequences them):

- **[docs/13 — multi-target](docs/13-multi-target-spec.md):** one source cell, three
  hash-attested artifacts (Z80 / RV32 / Thumb-1), provably identical behaviour, cycle
  certificates co-signed by real silicon.
- **[docs/14 — model-native cells](docs/14-model-native-cells-spec.md):** retrieval by
  execution (route a query by running the whole library against probes in one launch),
  decode-time wiring, trained invocation with cell reward organs, and the
  circuit-prosthetic experiment. The f32 GPU bank, CUDA, the library-launch cost, and
  per-cell step budgets ([docs/step-budget-amendment.md](docs/step-budget-amendment.md))
  are the owed items on the critical path.

---

## Origin

cell80 began as the compute-cell layer of
[`chuk-speccy`](https://github.com/chrishayuk/chuk-speccy) (a ZX Spectrum emulator +
agent platform) and was extracted, with full history, once the `.cell` artifact, CLI,
and MCP adapter made it a product in its own right. `chuk-speccy` now depends on cell80
for its Z80 core and compiled game logic — and the Z80, in turn, is now backend zero of
a family: the contract survives the chip.

## License

Dual-licensed under either [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your
option.
