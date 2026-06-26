# cell80 — Roadmap

cell80 is the deterministic **executable-tool-capsule** layer extracted from
[`chuk-speccy`](https://github.com/chrishayuk/chuk-speccy): a Z80 CPU core (`z80`), a
restricted-Rust → Z80 compiler (`rustz80`), and the cell micro-VM + tooling. The north star:

> Agents discover, inspect, compose, and run **millions of tiny executable tools** without
> loading their schemas into context. Each tool is a self-describing `.cell` cartridge.

## Built

**Compiler (`rustz80`).** `syn` frontend → typed IR → naive Z80 codegen (HL accumulator,
RAM scratch register file, ORG 0x8000). Subset: `u8`/`u16`/`u32`, arithmetic, `if`/`while`/
`for`/`loop`, early return, comparisons, arrays, `struct`/`enum`, functions, methods,
`poke`/`peek`/`inport`. The dialect is *also real Rust* → every program is differential-tested
against `rustc` on the emulator (`tests/diff.rs`).

**Cell micro-VM (`rustz80 --features cell`).**
- **Dual target** — `Spectrum48` (authentic, software mul/div) and `Cell` (Cell80: `ED FE`
  host traps for mul/div/fill/halt; a NOP on real hardware, so it never contaminates
  Spectrum output).
- **Runner** — compile-once/run-many; O(touched) reset between runs; `CellPool` recycles
  64 KiB buses. A decode-once **fast path** for straight-line cells (~0.05 µs/call batched).
- **Typed I/O** — `StateCell` (named typed inputs/outputs, no raw addresses); `entry_signature`
  derives the typed interface (params/ret/state) into the manifest.
- **`.cell` cartridge** — a named, versioned, self-describing artifact (manifest: id/summary/
  tags/entry/typed signature/source-hash/abi/caps + the compiled image).
- **Capabilities** — sandboxed by default (no raw memory / ports; bounded code + touched).
- **Honest cost** — `cycles` + a `trapped_ops` companion (traps are near-free in cycles, so
  count them — a reward-hack guard).
- **ABI v1 frozen** — `docs/09-cell80-abi.md`.
- **Index + host** — `CellIndex` (search by token overlap over tags/id/summary); `CellHost`
  (warm cached-runner sessions: `load → run* → unload`).
- **CLI `rustz80-cell`** — `run` (source) · `compile` (→ `.cell`) · `exec` (`.cell`) ·
  `inspect` · `index` · `search` · `serve` (persistent stdio session).
- **MCP front** — `cell80-py` (PyO3 `CellHost`) + `cell80-mcp` (`chuk-mcp-server`:
  `cell_search`/`cell_inspect`/`cell_list`/`cell_run`, a thin router over a warm host).
- **Trustworthiness** — host-vs-cell field-state differential; determinism + reset fuzzer
  (`tests/cell_fuzz.rs`).
- **Seed library** — `rustz80/cells/` (math/grid/scoring/validation/bench), all "excellent"
  tier (36–70 B, no caps), indexed + searchable.

## Next

- [ ] **CellGraph / inter-cell composition** — wire cells into a small static graph
  (planner→scorer→validator→decision; worker-swarm→reducer) that a session can run as one
  step. The composition layer.
- [ ] **Typed-state I/O over MCP** — `cell_run` currently takes register args; map named
  JSON inputs → struct field addresses via the signature so *state* cells (e.g. `manhattan`)
  are drivable by name. (The Rust + PyO3 + `StateCell` pieces exist.)
- [ ] **Grow the standard cell library** toward ~100 across the remaining categories
  (scoring/state/memory/data-structures/selection/sort/RNG/parsers/protocol).
- [ ] **Signed `i16`** — unblocks the scoring/delta cells (`x_y_delta`, signed `lerp`, risk
  deltas). Touches codegen/compares/div.
- [ ] **`bench` / `verify` / `trace` CLI verbs** + a persisted index (richer ranking:
  signature/capability/cost filters).
- [ ] **Crate identity & publish** — settle names (`cell80-core`/`-compiler`/`-cartridge`/
  `-cli`?) and publish to crates.io so `chuk-speccy` (and others) depend on released
  versions rather than a git/path dep.

## Origin & relationship to chuk-speccy

Extracted from chuk-speccy once all six "break-it-out" criteria were met (stable-enough cell
API, a `.cell` artifact, a standalone CLI, no Spectrum-emulator dependency in cell mode, a
30-second README, and a separable MCP adapter). The two stay mutually reinforcing:
`chuk-speccy` depends on cell80 for its Z80 core and compiled game/agent logic; cell80 still
emits authentic Z80/Spectrum output where needed. The full pre-extraction history lives in
this repo (filtered from chuk-speccy).
