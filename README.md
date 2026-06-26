# cell80

**Microsecond-scale, deterministic, sandboxed executable tool capsules for agents** — built
on a Z80 CPU core and a restricted-Rust → Z80 compiler.

A *cell* is a tiny, typed, deterministic function or state machine compiled to Z80 machine
code and run on a **flat-RAM Z80** — no ROM, no OS, no I/O, no syscalls — under a cycle
budget. The result: a self-describing executable artifact an agent can **discover, inspect,
run, compose, and discard** in microseconds. The pitch in one line:

> *millions of tiny tools, retrieved — not millions of tool schemas dumped into context.*

```
agent ──▶ cell_search("grid distance")   ──▶ a few brief manifests
      ──▶ cell_inspect("manhattan")       ──▶ run(...) -> u16, typed state
      ──▶ cell_run("gcd", [1071, 462])    ──▶ {result: 21, cycles, trapped_ops, halt}
```

## Why a Z80?

A 64 KiB flat address space and a tiny, fully-understood instruction set make a cell *small*
(tens of bytes of code), *deterministic* (cycle-exact, reproducible), and *cheap to sandbox*
(no ambient authority — it can only touch its own RAM). The same compiler frontend/IR also
targets an authentic ZX Spectrum, so cell logic and retro-game logic share one toolchain.

## The crates

| crate | what it is |
|-------|------------|
| **`z80`** (`cell80-z80`) | a cycle-accurate Z80 CPU core (no_std-friendly, dependency-free) |
| **`rustz80`** | a restricted-Rust → Z80 compiler: `syn` frontend → typed IR → Z80 codegen. The accepted subset is *also real Rust*, so every program is differential-tested against `rustc`. |
| **`rustz80 --features cell`** | the **cell micro-VM**: compile + run on a flat-RAM Z80. `.cell` cartridges (manifest + typed I/O signature), a compile-once/run-many `Runner` + `CellPool`, a decode-once fast path, an index/search, a warm `CellHost`, and the `rustz80-cell` CLI (`run`/`compile`/`exec`/`inspect`/`index`/`search`/`serve`). |
| **`cell80-py`** | PyO3 bindings — the warm `CellHost` as a Python class (built with maturin). |
| **`cell80-mcp`** | an MCP server (`chuk-mcp-server`) over a warm cell library: `cell_search`/`cell_inspect`/`cell_list`/`cell_run` — a thin router, not a tool-per-cell. |
| **`cell-bench`** | native-Rust / Wasmtime / cell / Python throughput + lifecycle comparison. |

## Quickstart

```bash
# compile a cell and run it (deterministic, sandboxed, headless)
echo 'fn run(a: u16, b: u16) -> u16 { let mut x=a; let mut y=b;
  while y != 0u16 { let t = x % y; x = y; y = t; } x }' > gcd.rs
cargo run -p rustz80 --features cell --bin rustz80-cell -- run gcd.rs --args 1071,462
# → result 21

# a self-describing .cell cartridge
rustz80-cell compile gcd.rs -o gcd.cell --id gcd --summary "Euclid's GCD" --tags math,bench
rustz80-cell inspect gcd.cell            # manifest + typed signature + capabilities

# a library you can search and run warm, in one process
rustz80-cell index  rustz80/cells
rustz80-cell search "grid distance" rustz80/cells
printf 'load gcd\nrun 0 1071,462\nquit\n' | rustz80-cell serve rustz80/cells
```

## Determinism & safety

Cells are **sandboxed by default** (no raw memory, no ports; bounded code + touched memory),
**deterministic** (same inputs → same result, cycles, and trapped-op count — fuzzed across
rerun / fresh / image-roundtrip / fast-vs-authentic), and **honest about cost**: `cycles`
plus a `trapped_ops` companion (so a reward function can't be gamed by routing work through
near-free host traps). The ABI is frozen at v1 (see `docs/09-cell80-abi.md`).

## Origin

cell80 began as the compute-cell layer of [`chuk-speccy`](https://github.com/chrishayuk/chuk-speccy)
(a ZX Spectrum emulator + agent platform) and was extracted once the `.cell` artifact, CLI,
and MCP adapter made it a product in its own right. `chuk-speccy` now depends on cell80 for
its Z80 core and compiled game/agent logic; cell80 still targets authentic Z80/Spectrum
output where needed.

## License

MIT OR Apache-2.0.
