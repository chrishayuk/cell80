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
$ rustz80-cell run score.rs --args 10,5 --json
{"abi":1,"entry":"run","result":155,"regs":[155,125,5],"cycles":327,"trapped_ops":2,
 "halt":"returned","code_bytes":47,"functions":1,"memory_touched":[[36864,36867],[65516,65519]]}
```

**47 bytes of code.** It returned `155`, took `327` T-states, touched 7 bytes of RAM, and
halted cleanly — every run reports exactly what it did and what it cost. The *same* `score.rs`
also compiles under `rustc`, so you can debug it as normal Rust; the two are kept honest by
differential testing.

Freeze it into a self-describing `.cell` cartridge:

```console
$ rustz80-cell compile score.rs -o score.cell --id score.v1 --summary "Score a candidate (x²+y²+3x)" --tags scoring,math
$ rustz80-cell inspect score.cell
cell `score.v1`  (abi 1, compiler 0.1.0)
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
$ rustz80-cell search "distance between grid points" cells/
indexed 8 cells; query `distance between grid points` → 2 match(es):
  manhattan — Manhattan distance between two grid points.  [grid, distance, spatial, score]  (Pts::run() -> u16)
  abs_diff  — Absolute difference |a - b| between two values.  [math, distance, diff]  (run(a: u16, b: u16) -> u16)
```

The loop an agent runs: **`search` → `inspect` → `run` → discard** — over a library that may
hold far more cells than belong in any context window. A good cell is one whose *manifest is
smaller than its usefulness*.

---

## Quick start

```bash
git clone https://github.com/chrishayuk/cell80 && cd cell80

# run a cell from source
cargo run -p rustz80 --features cell --bin rustz80-cell -- run rustz80/cells/gcd.rs --args 1071,462
# → result 21

# browse + search the seed library, then run one warm in a persistent session
cargo run -p rustz80 --features cell --bin rustz80-cell -- index  rustz80/cells
cargo run -p rustz80 --features cell --bin rustz80-cell -- search "validate a value in range" rustz80/cells
printf 'load gcd\nrun 0 1071,462\nquit\n' | cargo run -q -p rustz80 --features cell --bin rustz80-cell -- serve rustz80/cells
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
  (`returned` / `halted` / `cycle_budget` / `memory_limit`).
- **Deterministic.** Same inputs → same result, same cycle count, same touched-memory set —
  fuzzed across rerun, fresh instance, image round-trip, and the fast vs. authentic
  executor. Reproducible by design (no clocks, no RNG, no I/O).
- **Honest about cost.** Reports carry `cycles` *and* `trapped_ops` — host traps (mul/div)
  are near-free in T-states, so they're counted separately, which means a reward function
  can't be gamed by routing work through them.

The whole trust surface is *64 KiB of RAM + a cycle budget* — small enough to audit, and
the same for every cell.

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

At ~0.24 µs/call (fast path) a cell does **~4 million evaluations/second** — comfortably
inside an agent loop.

---

## Write a cell

The dialect is a bounded subset of real Rust — `u8`/`u16`/`u32`, arithmetic, `if`/`while`/
`for`/`loop`, arrays, `struct`/`enum`/`match`, functions and methods, `poke`/`peek`. A few
of the seed cells (`rustz80/cells/`):

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

---

## Connect it to an LLM (MCP)

`cell80-mcp` exposes a warm library over MCP as a thin **router** — not a tool per cell.
Four fixed tools (`cell_search` / `cell_inspect` / `cell_list` / `cell_run`) let a model
find and run the few cells it wants while the library stays out of context. Built on the
PyO3 binding `cell80-py` (the warm host as a Python class), the same Rust-core → PyO3 →
Python-MCP shape as the rest of the ecosystem.

```python
cell_search("grid distance")     # → a few brief manifests
cell_inspect("manhattan")        # → Pts::run() -> u16, typed state
cell_run("gcd", [1071, 462])     # → {result: 21, cycles, trapped_ops, halt}  (warm)
```

---

## The crates

| crate | what it is |
|-------|------------|
| **[`cell80-z80`](./z80)** | a cycle-accurate Z80 CPU core (`no_std`-friendly, dependency-free). Import name `z80`. |
| **[`rustz80`](./rustz80)** | the restricted-Rust → Z80 compiler: `syn` frontend → typed IR → Z80 codegen. Differential-tested against `rustc`. |
| **`rustz80 --features cell`** | the **cell micro-VM**: `.cell` cartridges, a compile-once/run-many `Runner` + `CellPool`, a decode-once fast path, `CellIndex`, the warm `CellHost`, and the `rustz80-cell` CLI. |
| **[`cell80-py`](./cell80-py)** | PyO3 bindings — the warm `CellHost` as a Python class (built with maturin). |
| **[`cell80-mcp`](./cell80-mcp)** | the MCP server over a warm cell library (`chuk-mcp-server`). |
| **[`cell-bench`](./cell-bench)** | the cross-runtime comparison (native / Wasmtime / cell / Python). |

The roadmap (`docs/roadmap.md`) tracks what's next: inter-cell composition (CellGraph),
typed-state I/O over MCP, a larger standard cell library, and signed `i16`.

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
