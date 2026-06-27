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
- **Z80 core conformance** — the foundation everything runs on passes the per-opcode
  **SingleStepTests** vectors **1,530,000/1,530,000** (initial→final state + cycle counts for
  every opcode/prefix incl. undocumented: base/CB/ED/DD/FD/DDCB/FDCB) and the **ZEXDOC**
  exerciser ROM; both fetch-on-demand (`z80-tests/`, see its README). Building this caught
  six real core bugs — the EI/IFF timing model, the undocumented repeat-flag rules for the
  LDIR/CPIR/INIR families, the DD/FD-prefixed SCF/CCF Q-latch, and `LD (IX+d),n` timing —
  now fixed.
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

1. **Agent eval harness — the headline milestone. → underway (`cell-eval/`).** Can an LLM
   `search → inspect → run` the right cell instead of writing code? Concrete cases: pick
   `manhattan` for grid distance, `range_check` for validation, `weighted_sum` for candidate
   scoring; compose `abs_diff + weighted_sum + clamp`; detect that *no* cell fits and ask
   for/compile one; prefer the safer/smaller/capability-free cell when two match; use
   reported `cycles` / `trapped_ops` / touched-memory to choose between implementations.
   This proves the real claim: *the consumer gets better because the cell is on the bus.*
   **Measure two numbers, not one:** (a) end-to-end **adoption** ("did it use a cell at
   all") and (b) **retrieval precision** ("given the query, is the right cell in top-k").
   They fail for different reasons — low adoption is often weak *steering* (system-prompt
   cueing), not bad retrieval. Hold the steering fixed, vary the library, and read precision
   directly, so a one-line preamble fix doesn't get misdiagnosed as a week of index tuning.
   - **Built so far** — `cell-eval/`, a standalone Python package driving the *same*
     `CellLibrary` the MCP server exposes. **Retrieval eval** is deterministic and runnable
     today (`cell-eval retrieval`): a paraphrase/adversarial dataset → precision@1 / hit@k /
     MRR, with a fixed-steering split (direct vs paraphrase vs adversarial). **Adoption eval**
     (`cell-eval adoption`) is a wired agent loop over an **OpenAI-compatible endpoint
     (Ollama by default)** — cell tools as function calls, scoring adoption + correctness +
     correct-via-cell; the steering prompt is a single held-fixed constant.
   - **Retrieval baseline (seed lib, k=5):** overall **P@1 0.74 / hit@3 0.90 / MRR 0.82**,
     but split it and the thesis-relevant number falls out: **direct P@1 1.00, paraphrase
     0.53, adversarial 0.50.** Token-overlap is perfect on the library's own words and a
     coin-flip under rewording — *"is this number within the allowed limits"* still misses
     `range_check` entirely. The paraphrase brittleness is **deferred to SOMA** (handled as a
     learning problem, not hand-tuned); this row is its regression guard, and the direct row
     guards against search regressing outright.
   - **Adoption baseline (gemma-4-26B-A4B via Ollama, 8 tasks):** **adoption 0.75, correct
     1.00, correct-via-cell 0.75.** The two non-adoptions (`max` of 17/42; `25 within 1–10`)
     were answered directly in one turn — the model shortcuts the cell when the arithmetic is
     trivial, *not* a retrieval miss (when it did reach for a cell it found the right one every
     time). This is exactly why the two numbers are tracked apart: correctness was perfect; the
     gap is adoption (steering), not retrieval.
   - **Next on this milestone** — push adoption past trivial-task shortcutting (steering /
     harder tasks); add typed-state tasks once item 2 lands; grow the dataset as the library does.
2. **Typed-state I/O over MCP** — the practical unlock. `cell_run` takes register args today;
   map named JSON fields → struct addresses via the signature so *state* cells (e.g.
   `manhattan`) are drivable by name. (The Rust + PyO3 + `StateCell` pieces exist.)
3. **Type-led index (the escape hatch from paraphrase brittleness).** Token-overlap is
   brittle — today *"is this number within the allowed limits"* retrieves `gcd` (its `number`
   tag), not `range_check`. But unlike a KnnStore *fact* (surface form only), a cell carries a
   **typed signature + capability/cost profile** — structured, verifiable metadata. "is this
   within limits" is a boolean-output, three-integer-bound query, a structurally stronger
   match for `range_check : (x,lo,hi)->bool` than `gcd : (a,b)->u16` *regardless of tags*. So
   make the **typed signature the primary ranking signal and embeddings the tiebreaker**, not
   the reverse — cell retrieval can beat fact retrieval *because the artifact is typed*. Gate
   it on a **paraphrased-query** stress test (a `.cell` is only useful if findable).
4. **`trace` / `verify` CLI** — every cell inspectable as *behaviour*, not just metadata.
5. **CellGraph / inter-cell composition** — wire cells into a small static graph
   (planner→scorer→validator→decision; worker-swarm→reducer). *Only after single-cell
   retrieval is reliable.*
6. **Grow the standard cell library** — toward ~100 cells, but driven by what the evals
   need, not by taxonomy.
7. **Signed `i16`** — unblocks scoring/delta cells (`x_y_delta`, signed `lerp`, risk deltas).

✓ **Published to crates.io** (`cell80-z80`, `rustz80` @ 0.2.0); `chuk-speccy` depends on the
released versions.

### `rustz80` frontend — features the chuk-speccy authoring-plane kit needs

A second, *frontend*-side ask on the shared compiler, driven by chuk-speccy's authoring plane
(its spec 08, track E): **the subset must widen so composable SDK kit types and assets compile
*pure*** — one `impl Game` source → a host build **and** a bootable tape. The pure-Snake seam is
closed today (`chuk-speccy-sdk/samples/snake_game.rs` compiles both ways and boots on the real
ROM), but only inside a narrow envelope. Each item is a concrete blocker found while shaping the
SDK kit; the `file:line` is `rustz80` 0.2.0. These sit **below the agent-tool arc above** (the
eval is the gate, not VM/compiler features) but are tracked here since the compiler lives here.

- **Nested struct fields + field-of-field access** (`self.sprite.x`, a game-state field that is
  itself a struct). Blocked at `lower/layout.rs:103` (a `Type::Path` field is always 1 slot — no
  struct recursion) and `lower/expr.rs:235` (*"nested struct fields are not supported"*). This is
  the gate on the whole composable kit — `Sprite`/`Actor`/`TileMap`/`Hud` as fields. Today only a
  `[Struct; N]` element array carries sub-structure (`a[i].x` via `elem_field_addr`).
- **Wider persisted struct fields — `u32` and signed `i16`.** Struct fields are 16-bit slots
  (`layout.rs:103`); `u32` exists only as a *local* in the `HL:DE` pair, so it cannot persist in
  state. This forced the pure Snake's `u32` xorshift `Rng` down to a `u16` xorshift, and blocks
  signed state. (Pairs with **Next #7, signed `i16`** — same widening, for both cells and games.)
- **`[u8; N]` byte-array fields.** Only `[u16; N]` (and `[Struct; N]`) array fields lay out
  (`layout.rs:104`, `is_u16`); a byte array falls into the struct-element path and errors.
  Unblocks compact byte buffers — tile rows, packed grids, string bytes — without 2× `u16` waste.
- **`&CONST → addr` — a const-data section** (lay `const` bytes for tile bitmaps / strings into
  the image and resolve `&TILE` / `&str` to that address, so a handle method receives a pointer).
  The single biggest unlock: it lets `Frame::tile(&Tile)` and `Frame::text(&str)` route **by
  address**, i.e. real sprite bitmaps **and a text HUD** in pure games, and gives the
  `chuk-speccy-assets` tile pipeline a pure target. Today pure games draw only data-free solid
  cells (`fill_cell`, colour passed by value).
- **(convention, not a compiler feature) one struct definition, both sides of the dial.** A type
  used in a *pure* game today must be prelude-provided, so the host can't also `use` it without a
  redeclaration clash — pure samples are limited to the prelude's `Frame`/`Input`/`Colour`/
  `Button` plus primitives and arrays. A shared-source mechanism (extending the game-sample
  pattern) would let a `Sprite` be defined once and compile host **and** pure.

Until these land, chuk-speccy's *pure* kit stays inside the envelope (solid-cell sprites, `u16`
RNG, `[u16; N]` pools, no text HUD); its **host** SDK and asset tooling proceed independently.

### Design rule for SOMA / RL: cost is a **gate**, not a gradient
Keep `cycles`/`trapped_ops` as **constraints** — gate trap-heavy cells out, halt on budget —
**never as a reward-shaping term**. The moment cost enters the *reward*, the trap-routing
gaming surface reopens *and* you're forced to pick the `trapped_ops` weight the ABI doc admits
it can't make faithful (real-Z80 cost vs host-trap ≈4). Keeping cost a constraint sidesteps
the weight choice entirely and keeps the gaming surface shut. Decide this before the first
reward function ships, not after.

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
