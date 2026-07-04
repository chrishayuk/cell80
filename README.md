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
indexed 145 cells; query `distance between grid points` → 10 match(es):
  manhattan — Manhattan distance between two grid points (typed state).  [grid, distance, spatial, score, navigation]  (Pts::run() -> u16)
  euclid_sq — Squared Euclidean distance between two grid points: dx*dx + dy*dy (no sqrt).  [grid, distance, euclidean, squared, spatial]  (Pts::run() -> u16)
  chebyshev — Chebyshev (chessboard) distance between two grid points: max(|dx|, |dy|).  [grid, distance, chebyshev, chessboard, spatial]  (Pts::run() -> u16)
  ...                                                                   (abs_diff and 6 more, lower-ranked)
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

Retrieval on the 145-cell library (`cargo run --example retrieval_compare -p cell80`): the
default index is now **TF-IDF** (word + char-3-gram cosine) — **direct P@1 0.95**, **paraphrase
0.44** — a few points over the old token overlap, but paraphrase stays well under direct as confusable
siblings multiply (twenty-five families: predicates, bounds, distance, number theory, bit ops,
hashing, …). A **type-led** re-rank by the cell's *behaviour* (is it a predicate? — learned from
the corpus, not hardcoded) was measured **neutral** on this set, for an honest reason: the
residual misses are *same-shape siblings* (`min`/`max`, `gcd`/`lcm`, `manhattan`/`chebyshev`) no
text or signature signal can separate. The lever for those is **behavioural I/O-example routing**
(`cell_route_by_example`): on `(3,7)→3` only `min` matches, not `max` — selection grounded in
what the cell *does*, phrasing- and language-independent.
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

# run a cell from source
cargo run -p cell80 --bin cell80 -- run cell80/cells/gcd.rs --args 1071,462
# → result 21

# browse + search the seed library, then run one warm in a persistent session
cargo run -p cell80 --bin cell80 -- index  cell80/cells
cargo run -p cell80 --bin cell80 -- search "validate a value in range" cell80/cells
printf 'load gcd\nrun 0 1071,462\nquit\n' | cargo run -q -p cell80 --bin cell80 -- serve cell80/cells
```

CLI verbs: `run` (source) · `compile` (→ `.cell`) · `exec` (a `.cell`) · `inspect` ·
`index` · `search` · `serve` (a persistent warm session). Same commands, drivable by an
agent over JSON.

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
`struct`/`enum`/`match`, functions and methods, generics (monomorphized), top-level `const`
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
| **[`rustz80`](./rustz80)** | the restricted-Rust → Z80 compiler: `syn` frontend → typed IR → Z80 codegen. Differential-tested against `rustc`. |
| **[`cell80`](./cell80)** | the **cell micro-VM + tooling** (built on `rustz80`): `.cell` cartridges, a compile-once/run-many `Runner` + `CellPool`, a decode-once fast path, `CellIndex`, the warm `CellHost`, typed-state I/O, host-routed `CellGraph` composition, and the `cell80` CLI. |
| **[`cell80-py`](./cell80-py)** | PyO3 bindings — the warm `CellHost` as a Python class (built with maturin). |
| **[`cell80-mcp`](./cell80-mcp)** | the MCP server over a warm cell library (`chuk-mcp-server`). |
| **[`cell-eval`](./cell-eval)** | the agent eval harness — retrieval precision + LLM adoption + composition (does an agent find, run, and *compose* the right cells instead of writing code?). |
| **[`cell-bench`](./cell-bench)** | the cross-runtime comparison (native / Wasmtime / cell / Python). |
| **[`z80-tests`](./z80-tests)** | the Z80 conformance harness — SingleStepTests vectors + ZEXDOC. |

The roadmap (`docs/roadmap.md`) tracks the agent eval harness, typed-state I/O over MCP
(done), the **standard library** (done — **145 cells** across 25 families incl. wide u32-in-state siblings, plus the compiler
ergonomics that make predicates/bitops one-liners and a **shared-kernel prelude + dead-code
elimination** so cells reuse `gcd`/`imin`/`iabs_diff`/… instead of re-implementing them), and
**host-routed `CellGraph` composition** (cells wired into a static, type-checked graph the host
validates before running) with the **`cell_compose`** pipeline helper that lets a small model
actually author one (measured: composition `used_graph` 0.00 → `used_pipeline` 0.50).
The execution plan is phased in [`docs/roadmap-phases.md`](docs/roadmap-phases.md):
**Phase 0 (shipped)** closed the determinism contract — recursion rejected at compile time,
the Cell trap path differential-tested against rustc, `/ 0` a typed halt, the dialect
semantics written down — then the LLM-facing compiler (if/match expressions, diagnostics,
signed `i16` — shipped), retrieval as the product, trust (signed cells, escalation contract,
memoization), and codegen stage 2 (the symbolic `Ins` layer + the measured peephole —
shipped, −4.3 % corpus code size; u32 array elements remain).

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
