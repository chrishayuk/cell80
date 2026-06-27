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
- **Trustworthiness** — host-vs-cell field-state differential; determinism + reset fuzzer;
  and the **named round-trip fuzz** (`state_named_roundtrip_fuzz`): 500 random inputs set
  *by name* → run → read inputs+outputs back *by name* vs a host oracle — the B3
  field↔memory↔field seam as one property, not two halves (`tests/cell_fuzz.rs`).
- **Seed library** — `rustz80/cells/` (math/grid/scoring/validation/bench), all "excellent"
  tier (36–70 B, no caps), indexed + searchable.

## Positioning

> **cell80 is not a faster Wasm. It is a manifest-addressable executable micro-tool format
> for agents.** A `.cell` is closer to an *executable index card* than a plugin: a tiny
> deterministic behaviour with a typed signature, a hash, a cost surface, a capability
> policy, and bounded execution. *A tool should not need a server, a process, or a page of
> schema if it's only 47 bytes of behaviour.*

The proof of the thesis isn't VM features — it's whether **an agent reliably retrieves and
runs the right cell instead of writing Python**. That's the next gate.

## Next

The VM is proven; the open problem is **library semantics + discovery quality**. Ordered so
single-cell retrieval works before composition, and the library grows by eval need:

1. **Agent eval harness — the headline milestone.** Can an LLM `search → inspect → run` the
   right cell instead of writing code? Concrete cases: pick `manhattan` for grid distance,
   `range_check` for validation, `weighted_sum` for candidate scoring; compose
   `abs_diff + weighted_sum + clamp`; detect that *no* cell fits and ask for/compile one;
   prefer the safer/smaller/capability-free cell when two match; use reported `cycles` /
   `trapped_ops` / touched-memory to choose between implementations. This proves the real
   claim: *the consumer gets better because the cell is on the bus.*
2. **Typed-state I/O over MCP** — the practical unlock. `cell_run` takes register args today;
   map named JSON fields → struct addresses via the signature so *state* cells (e.g.
   `manhattan`) are drivable by name. (The Rust + PyO3 + `StateCell` pieces exist.)
3. **Persisted manifest index with richer ranking + paraphrase robustness** — tags alone
   aren't enough, and token-overlap is brittle: today *"is this number within the allowed
   limits"* retrieves `gcd` (matched its `number` tag), not `range_check`. Retrieval is
   load-bearing for the whole pitch and precision is the unsolved problem — determinism
   doesn't save you from confidently running the *wrong* cell. Rank on signature, I/O types,
   capability + cost profile, examples, and "negative affordances"; **stress-test on
   paraphrased queries** as a gate (a `.cell` is only useful if its manifest is *findable*).
4. **`trace` / `verify` CLI** — every cell inspectable as *behaviour*, not just metadata.
5. **CellGraph / inter-cell composition** — wire cells into a small static graph
   (planner→scorer→validator→decision; worker-swarm→reducer). *Only after single-cell
   retrieval is reliable.*
6. **Grow the standard cell library** — toward ~100 cells, but driven by what the evals
   need, not by taxonomy.
7. **Signed `i16`** — unblocks scoring/delta cells (`x_y_delta`, signed `lerp`, risk deltas).

✓ **Published to crates.io** (`cell80-z80`, `rustz80` @ 0.2.0); `chuk-speccy` depends on the
released versions.

## Non-goals — keep it boring

The magic is that a cell is tiny, inspectable, bounded, deterministic, and *almost boring*.
The moment it looks like a mini-OS or a Wasm competitor it loses its shape. So, deliberately
**out of scope**: filesystem / network / general syscalls, ports & ROMs as a feature,
ambient authority, a growing instruction set or "general tiny computer" surface, and being a
general sandboxed-compute runtime (that's Wasm's job). Heavy or general compute belongs in
Wasm/native/Python; cell80 stays the *small contract*.

## Origin & relationship to chuk-speccy

Extracted from chuk-speccy once all six "break-it-out" criteria were met (stable-enough cell
API, a `.cell` artifact, a standalone CLI, no Spectrum-emulator dependency in cell mode, a
30-second README, and a separable MCP adapter). The two stay mutually reinforcing:
`chuk-speccy` depends on cell80 for its Z80 core and compiled game/agent logic; cell80 still
emits authentic Z80/Spectrum output where needed. The full pre-extraction history lives in
this repo (filtered from chuk-speccy).
