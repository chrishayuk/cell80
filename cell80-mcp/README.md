# cell80-mcp

An MCP server over a **warm library of deterministic micro-tools** (cells) compiled by
`cell80`. It's a thin **router**, not a tool-per-cell: the library can hold millions
of cells, but the model only ever sees the handful it searches up.

```
agent ──▶ cell_search("grid distance")   ──▶ a few brief manifests
      ──▶ cell_inspect("manhattan")       ──▶ typed signature
      ──▶ cell_run("gcd", [48, 36])       ──▶ {result: 12, cycles, trapped_ops, halt}
```

## Why a server (not the CLI)

A per-invocation CLI spawns a fresh process (~10 ms) and throws the warm runner away every
call — which defeats the ~0.05 µs warm execution. This server holds the host (index + warm
runners) in **one process**: `cell_run` keeps a runner warm per cell across calls.

It's built on `chuk-mcp-server` over the PyO3 binding `cell80-py` (which wraps the Rust
`cell80::CellHost`) — the same Rust-core → PyO3 → Python-MCP pattern as
`chuk-mcp-spectrum`/`zxspec-py`. The session/warmth lives in `CellLibrary`; the tools carry
no policy, so a socket daemon or another transport could wrap the same bodies.

## Tools

| tool | what it does |
|------|--------------|
| `cell_search(query, limit=10)` | rank the library by relevance → brief manifests |
| `cell_inspect(id)` | full manifest: typed signature (params/ret/state), abi, hash |
| `cell_list()` | every cell (brief) |
| `cell_run(id, args=[])` | run a plain cell with `u16` register args → result + cost + halt |
| `cell_run(id, fields={…})` | drive a **state** cell by name → result + full post-run `state` + cost |
| `cell_compose(steps, inputs={…})` | **compose the easy way**: a pipeline of `{cell, args}` (positional; `"$N"` = step N's result), ports resolved from the manifest — no wires → outputs + trace |
| `cell_graph_run(graph, inputs={…})` | **compose** cells: run a host-routed `CellGraph` (the full manifest, for DAGs; type-checked before running) → outputs + per-node trace |

## Run

```bash
maturin build -m ../cell80_py/Cargo.toml      # build the PyO3 binding wheel
pip install ../cell80_py/target/wheels/cell80_py-*.whl
pip install -e .
CELL_LIBRARY=../cell80/cells CELL_STDIO=1 cell80-mcp   # MCP over stdio
```

`CELL_LIBRARY` selects the cell directory (default `cell80/cells`); without `CELL_STDIO`
it listens on HTTP (`CELL_HOST`/`CELL_PORT`).
