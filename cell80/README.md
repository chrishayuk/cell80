# cell80

**Deterministic, sandboxed executable tool capsules for agents** — the cell micro-VM and
tooling, built on the [`rustz80`](../rustz80) compiler and the [`cell80-z80`](../z80) CPU core.

A *cell* is a tiny function or state machine, written in a subset of Rust that is *also* real
Rust, compiled to Z80 and run on a flat-RAM virtual machine: no OS, no filesystem, no network,
no syscalls, no ambient authority. It touches only its own 64 KiB of RAM for a bounded number
of cycles, then returns a number. That whole sandbox fits in your head.

This crate is the layer *above* the compiler: `rustz80` stays a pure restricted-Rust → Z80
compiler; **cell80** is everything that makes a compiled program a discoverable, runnable,
composable tool.

## What's here

- **`CellProgram` / `Runner` / `CellPool`** — compile once, instantiate many cheap runners;
  O(touched) reset between runs; a decode-once fast path (~0.05–0.25 µs/call warm).
- **`.cell` cartridge** (`Cartridge` / `Manifest`) — a named, versioned, self-describing
  artifact: id, summary, tags, entry, typed signature (params/ret/state), source hash, ABI +
  format version, capability policy, the compiled image, and (v3) the state field addresses.
- **`CellHost` + `CellIndex`** — a warm session: `search` → `inspect` → `load` → `run` many →
  `unload`, with relevance search over the manifests.
- **Typed state I/O** (`StateCell`, `CellHost::run_state`) — drive a state cell **by field
  name** (JSON↔state), no raw addresses.
- **`CellGraph`** — static, **host-routed** composition: wire one cell's typed output into
  another's typed input. The host validates the whole graph (port types, completeness,
  acyclicity) **before a single cycle runs**; cells never see each other.
- **`CellConfig`** — capability gates (raw memory / ports off by default) + size/cycle caps.
- **the `cell80` CLI** — `run` · `compile` · `exec` · `inspect` · `index` · `search` ·
  `serve` · `graph`.

```console
$ cell80 run cells/gcd.rs --args 1071,462           # → result 21
$ cell80 search "grid distance" cells               # rank the library
$ cell80 graph graphs/move_ranker.json cells --input x1=3,y1=4,x2=10,y2=8,risk=2,cost=1
```

See the [top-level README](../README.md) for the vision, benchmarks, and the agent/MCP story,
and [`docs/09-cell80-abi.md`](../docs/09-cell80-abi.md) for the ABI + cartridge format.

## License

Dual-licensed under either [MIT](../LICENSE-MIT) or [Apache-2.0](../LICENSE-APACHE), at your option.
