# cell80 — Roadmap

cell80 is the deterministic **executable-tool-capsule** layer extracted from
[`chuk-speccy`](https://github.com/chrishayuk/chuk-speccy): a Z80 CPU core (`z80`), a
restricted-Rust → Z80 compiler (`rustz80`), and the cell micro-VM + tooling. The north star:

> Agents discover, inspect, compose, and run **millions of tiny executable tools** without
> loading their schemas into context. Each tool is a self-describing `.cell` cartridge.

## Built

**Compiler (`rustz80`).** `syn` frontend → typed IR → naive Z80 codegen (HL accumulator,
RAM scratch register file, ORG 0x8000). Subset: `u8`/`u16`/`u32`, arithmetic, `if`/`while`/
`for`/`loop`, early return, arrays, `struct`/`enum`, functions, methods, `poke`/`peek`/`inport`.
**Booleans:** comparisons (`< <= > >= == !=`) work both as branch conditions *and* as `0`/`1`
**values** (`(a < b) as u16`), short-circuit `&&` / `||`, and bit shifts take a **runtime**
amount (`x << bit`) — so predicates and bit ops are one-liners. **Dead-code elimination**
(`compile_file_pruned`) keeps only functions reachable from the roots — the cell layer uses it
to prepend a shared-kernel prelude and prune whatever a cell doesn't call. The dialect is *also
real Rust* → every program is differential-tested against `rustc` on the emulator
(`tests/diff.rs`).

**Cell micro-VM (the `cell80` crate, built on `rustz80`).**
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
- **Index + host** — search defaults to **`TfidfIndex`** (IDF-weighted word + char-3-gram
  cosine, lazily rebuilt in `CellHost`; `CellIndex` token-overlap kept as the baseline), plus
  **behavioural routing** (`CellHost::route_by_examples` over `rank_by_examples` — find a cell by
  the I/O examples it reproduces); `CellHost` warm cached-runner sessions (`load → run* →
  unload`).
- **CLI `cell80`** — `run` (source) · `compile` (→ `.cell`) · `exec` (`.cell`) ·
  `inspect` · `index` · `search` · `serve` (persistent stdio session, with `route <in>=<out>`) ·
  `graph` (run a `CellGraph` manifest).
- **MCP front** — `cell80-py` (PyO3 `CellHost`) + `cell80-mcp` (`chuk-mcp-server`): a thin
  router over a warm host — `cell_search` / **`cell_route_by_example`** / `cell_inspect` /
  `cell_list` / `cell_run` (positional **or** named `fields` for state cells) / `cell_compose` /
  `cell_graph_run` (compose cells).
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
- **Standard library** — `cell80/cells/` is now **98 cells**: the 8 originals plus ~12
  confusable families — **predicates**, **safe arithmetic**, **bounds**, **percent/ratio**,
  **ranking/stats**, **bit/mask**, **number theory** (`lcm`, `is_prime`, `isqrt`,
  `factor_count`, `pow_mod`, …), **distance** (`chebyshev`, `euclid_sq` — state-cell siblings of
  `manhattan`), **bit/encoding** (`rotl16`, `reverse_bits`, `bit_length`, `swap_bytes`, …),
  **hashing** (`hash_pair`, `fnv1a_step`, `crc8_step`, `mix16`), and **bucketing/conversion**.
  All indexed + searchable, with a per-cell host-oracle (`cell80/tests/library.rs`) and
  direct/paraphrase/adversarial retrieval rows.
- **Modular cells — shared kernel prelude + DCE.** Cells reuse a small prelude (`gcd`, `imin`,
  `imax`, `iabs_diff`, `isqrt`, `clamp_to`) instead of duplicating it — `lcm` calls `gcd`,
  `chebyshev` calls `iabs_diff`/`imax`. The prelude is appended to every cell and dead-code
  elimination prunes the kernels a cell doesn't reach, so a cartridge carries only what it uses
  (a kernel-free cell stays byte-identical). See `docs/library-growth.md` (packs, the
  contribution rule, and the modularity rules).
- **Typed-state I/O** — drive a state cell **by field name** end-to-end (`CellHost::run_state`
  → PyO3 → MCP `cell_run(fields=…)`); the scalar field addresses are baked into the manifest
  (`state_addrs`) so a host or a peer cell drives by name with no source.
- **`CellGraph` (host-routed composition)** — wire cells into a static graph (`cell80/src/
  graph.rs`); the host **type-checks the whole graph before a single cycle runs**, then runs
  nodes in topological order routing typed values, with a combined trace. A JSON manifest is
  drivable three ways (the `cell80 graph` CLI, PyO3 `run_graph`, MCP `cell_graph_run`). Cells
  never see each other — the bus is the host's.
- **Eval harness** (`cell-eval`) — measures the whole arc over the *same* warm library:
  deterministic **retrieval**, LLM **adoption**, and **composition** (see **Next** for the
  rationale + baselines).
- **CI + release** — matrix CI (rust + python on Linux/macOS/Windows); a **tag-triggered,
  test-gated** crates.io publish (`cell80-z80` → `rustz80` → `cell80`).

## Positioning

> **cell80 is not a faster Wasm. It is a manifest-addressable executable micro-tool format
> for agents.** A `.cell` is closer to an *executable index card* than a plugin: a tiny
> deterministic behaviour with a typed signature, a hash, a cost surface, a capability
> policy, and bounded execution. *A tool should not need a server, a process, or a page of
> schema if it's only 47 bytes of behaviour.*

The proof of the thesis isn't VM features — it's whether **an agent reliably retrieves, runs,
and composes the right cells instead of writing Python**. That's now *measured* (the eval
harness, item 1); the open problems are discovery quality and graph-authoring, not the VM.

## Next

The VM is proven and the eval loop is in place (items 1, 2, 5 ✓). The open problem is now
**discovery quality** (item 3): TF-IDF is the default index and paraphrase is still a coin-flip,
with the lever now **behavioural I/O-example routing**, not more text. **cell80 owns this** —
retrieval, the type-led index, the graph-authoring surface, the verifier, and the optional
synthesis mode are cell80's discovery + agent surface, *not* SOMA's; SOMA only enters to
**schedule** these capabilities as fast/slow organs under pressure. Numbered by theme, not
strictly by sequence; the library grows by eval need:

1. **Agent eval harness — the headline milestone. ✓ done (`cell-eval/`).** Can an LLM
   `search → inspect → run` the right cell instead of writing code? Concrete cases: pick
   `manhattan` for grid distance, `range_check` for validation, `weighted_sum` for candidate
   scoring; compose `abs_diff + weighted_sum + clamp`; detect that *no* cell fits and ask
   for/compile one; prefer the safer/smaller/capability-free cell when two match; use
   reported `cycles` / `trapped_ops` / touched-memory to choose between implementations.
   This proves the real claim: *the consumer gets better because the cell is on the bus.*
   **Measure three numbers, each failing differently:** (a) **retrieval precision** ("given
   the query, is the right cell in top-k"), (b) **adoption** ("did it use a cell at all"), and
   (c) **composition** ("did it *wire several cells together*", item 5). They fail for
   different reasons — low adoption is often weak *steering* (system-prompt cueing), not bad
   retrieval. Hold the steering fixed, vary the library, and read precision directly, so a
   one-line preamble fix doesn't get misdiagnosed as a week of index tuning.
   - **Built** — `cell-eval/`, a standalone Python package driving the *same* `CellLibrary`
     the MCP server exposes, with three subcommands sharing one held-fixed-steering agent loop.
     **`retrieval`** is deterministic (no model): a paraphrase/adversarial dataset →
     precision@1 / hit@k / MRR, split direct vs paraphrase vs adversarial. **`adoption`** and
     **`composition`** are agent loops over an **OpenAI-compatible endpoint (Ollama by
     default)** — cell tools (incl. `cell_graph_run`) as function calls.
   - **Retrieval (98-cell library, k=5, `examples/retrieval_compare`):** the default index is
     now **TF-IDF** (word + char-3-gram cosine, lazily rebuilt in `CellHost`) — **direct P@1
     0.97, paraphrase 0.45, adversarial 0.31** — a few points over the old token overlap, but
     paraphrase stays a coin-flip as the ~12 confusable families multiply. Growing 8 → 98 cells
     made retrieval *harder on purpose*: the direct misses are now sibling collisions (`gcd` vs
     `gcd3`/`lcm`, `manhattan` vs `chebyshev`/`euclid_sq`). A **type-led** re-rank by behaviour
     was measured **neutral** on this set — those siblings are *same-shape*, invisible to text
     *and* signature; the language-independent lever for them is behavioural I/O-example routing
     (item 3). The direct row guards against search regressing outright.
   - **Adoption baseline (gemma-4-26B-A4B via Ollama, 8 tasks):** **adoption 0.75, correct
     1.00, correct-via-cell 0.75.** The two non-adoptions (`max` of 17/42; `25 within 1–10`)
     were answered directly in one turn — the model shortcuts the cell when the arithmetic is
     trivial, *not* a retrieval miss (when it did reach for a cell it found the right one every
     time). This is exactly why the two numbers are tracked apart: correctness was perfect; the
     gap is adoption (steering), not retrieval.
   - **Status** — all three modes ship and run against a local model. The retrieval lever is
     now **behavioural routing** (item 3 — the type-led text re-rank measured neutral); the
     composition lever was **`cell_compose`** (item 5). (Composition baseline + the `used_graph`
     finding live under item 5.)
2. **Typed-state I/O over MCP — ✓ done.** `cell_run` now takes a `fields` object ({name:int})
   for state cells: named fields → struct addresses (baked into the manifest at compile time
   as `state_addrs`, so a warm host or a peer cell drives by name with no source), run, and the
   full post-run state read back. `manhattan` and friends are drivable by name end-to-end
   (`CellHost::run_state` → PyO3 `run_state` → MCP `cell_run(fields=…)`), and manhattan is back
   in the adoption tasks. **This is also the wiring substrate for the networked graph (edge 0):
   a CellGraph edge is one cell's named output fed into another's named input.**
3. **Type-led / behavioural discovery — measured; the lever is behaviour, not text. ✓ shipped.**
   The bet was that a cell's typed signature could out-rank text for paraphrases. Built and
   measured (`cell80/src/typeled.rs`, `examples/retrieval_compare`): a **type-led** re-rank by
   behavioural **predicate-ness** — learned from the corpus as smoothed log-odds, *no hardcoded
   vocabulary*, labels free from running the cell — is **neutral** vs plain TF-IDF on the 98-cell
   set. The honest reason: the residual misses are *same-shape siblings* (`min`/`max`, `gcd`/
   `lcm`, `manhattan`/`chebyshev`) that an arity/signature signal can't separate, and inferring
   the target shape from a paraphrase hits the same vocabulary gap. What *does* separate them is
   **behaviour**: on `(3,7)→3` only `min` matches, not `max`. So **behavioural I/O-example
   routing** is now first-class — `CellHost::route_by_examples`, a `route` serve verb, and the
   **`cell_route_by_example`** MCP tool over `rank_by_examples` (`cell80/src/fingerprint.rs`):
   phrasing- and language-independent selection grounded in what the cell *does*. (`TypeLedIndex`
   stays as the principled re-ranker and the home for further structural axes.) **Next** — richer
   behavioural fingerprints (output cardinality, monotonicity) and discriminating-probe selection,
   then let the model *learn* to pick probes (where SOMA would schedule, not own).
4. **`trace` / `verify` CLI** — every cell inspectable as *behaviour*, not just metadata.
5. **CellGraph / inter-cell composition — core built; this is the chase.** Wire cells into a
   small static graph (planner→scorer→validator→decision; worker-swarm→reducer).
   - **Built** — `cell80/src/graph.rs`: a `CellGraph` (nodes = cells, wires = typed
     feeds from constants / external inputs / another node's output, named outputs). The host
     **validates the whole graph before a single cycle runs** — every wire's source-port type
     must match its destination-port type (the win that only typed artifacts allow), every
     value-cell input must be fed, ports must exist, and the graph must be acyclic — then runs
     nodes in topological order, routing typed values between them and recording a combined
     per-node trace. **Cells never see each other: the bus is the host's, no
     sockets/files/syscalls** (the non-goals hold). First slice runs end-to-end:
     `manhattan → weighted_sum → clamp` (`cell80/tests/graph.rs`).
   - **Agent surface — ✓ done.** A JSON graph **manifest** (`CellGraph::from_json`), drivable
     three ways over the *same* warm host: the `cell80 graph` CLI verb, PyO3
     `CellHost.run_graph`, and the MCP **`cell_graph_run`** tool. An agent authors and runs a
     graph as data and gets back the outputs + a combined per-node trace.
   - **Composition eval — ✓ done.** `cell-eval composition`: tasks that need ≥2 cells wired
     together, the agent given the `cell_graph_run` tool, scored on **composed /
     correct / correct-via-composition** (held-fixed steering, like adoption). The capstone —
     it measures whether the consumer *builds* a tool from several, not just *finds* one.
   - **Baseline → the `cell_compose` fix (granite4.1:3b) — ✓ graph-authoring gap closed.** With
     the raw `cell_graph_run` manifest only, the model **composed 0.50 / correct 0.83 but
     `used_graph` 0.00** — it chains `cell_run` and never authors the wire-level JSON (too much
     for a 3B). That drove **`cell_compose`** (built): an ordered pipeline of `{cell, args}` with
     positional args (`"$N"` = step N's result), ports resolved from each cell's manifest — no
     wires, no port names. The same model now **composes via a pipeline in half the tasks**
     (`used_pipeline` 0.50, raw `used_graph` still 0.00, composed **0.79**, correct **0.93**). So
     graph-authoring **ergonomics** was the lever, exactly as predicted — not the VM. (adoption
     1.00 / correct 1.00 on single cells throughout.)
   - **Next** — non-linear (DAG) authoring sugar, let the model *learn* to author graphs
     (cell80's authoring surface; SOMA would *schedule* these, not own them), then a live
     **CellBus** (publish typed event → route to interested cells → commit).
   *(Reordered ahead of retrieval: a static, host-authored graph needs no retrieval — that's
   for when an agent authors graphs. It rests on item 2's named typed I/O, which is the edge.)*
6. **Grow the standard cell library — two waves ✓ (98 cells).** `cell80/cells/`: predicates,
   safe arithmetic, bounds, percent, ranking/stats, bit/mask, number theory, distance, encoding,
   hashing, bucketing/conversion — each with retrieval rows + a host-oracle test, and now
   **modular** via the shared kernel prelude + DCE (see `docs/library-growth.md`). Keep going
   (driven by what the evals need, not taxonomy) — next: packing/BCD, multi-weight scoring &
   choice (state cells), vector dot/norm, time/budget arithmetic, and stateful/RNG cells
   (`lcg_next`/counters/`ema_update`).
7. **Signed `i16`** — unblocks scoring/delta cells (`x_y_delta`, signed `lerp`, risk deltas).
   (Compiler ergonomics groundwork landed: `bool` flags + unary `!`.)
8. **Experimental: outcome-specified synthesis** (`cell80/src/synth.rs`) — the *inverse* of
   `CellGraph`: given input→output **examples**, search over short cell chains and verify
   candidates by execution (the verifier is the engine). A deliberately separate mode from
   normal `search → inspect → run` tool-calling — *given behaviour, discover a graph* — kept out
   of the main pitch. Honestly gated: a learned value heuristic only *ties* the hand Hamming
   heuristic at equal budget so far (a kill gate, not a given). See `examples/composition_eval`.

✓ **Published to crates.io** (`cell80-z80`, `rustz80`, `cell80`, via the tag-triggered publish
job in CI); `chuk-speccy` depends on the released versions. (rustz80 0.5.0 dropped the `cell`
feature — the cell layer is now the `cell80` crate.) **0.6.0** added the compiler ergonomics
(bool-as-value, `&&`/`||`, runtime shifts) + dead-code elimination, the 98-cell standard library
with a shared kernel prelude, and Spectrum-side DCE in `compile_to_tap`. **0.7.0** adds the
`cell_compose` pipeline graph-authoring helper, a single-call inliner, and the cell layer's
learned retrieval router + program-synthesis modules. **0.8.0** makes **TF-IDF the default
search index**, adds **behavioural I/O-example routing** (`cell_route_by_example`) as the
language-independent lever for same-shape sibling confusions, the (experimental, measured-neutral)
type-led re-ranker, and makes codegen **return `Result`** instead of panicking on over-budget.

### `rustz80` backend — Stage 2 quality + wider state (the shared multiplier)

Distinct from the frontend asks below (chuk-speccy's critical path): this backend work pays off
for *both* consumers — cells get headroom under the manifest-smaller-than-usefulness line and
lower decode cost (and since `cycles` is a **constraint, not a reward**, cheaper cells are pure
upside — no gaming surface reopens); games get room under the 4 KB code ceiling that just bit
`chase`. The ISA was never the ceiling: `ED FE` makes cell80 *Z80 for control flow + host traps
for disproportionate primitives*, so the test for any new width is the **contract — bounded +
honest cost** — not "can the chip do it."

**Stage 2 — codegen quality.** The blocker is structural, not effort: codegen emits raw bytes
straight into a `Vec<u8>` (`a.byte(0xE5)`), so instruction boundaries are gone and there is no
seam for a quality pass.

- **Instruction-IR seam (the keystone).** Interpose a thin `Ins` list between codegen and
  `finish()` — an enum with **symbolic operands** (labels / slot refs / call targets stay
  symbolic, not byte offsets; the runtime appended as `Ins::Blob(&[u8])`). The address model
  inverts: PC + scratch assignment moves to a final pass *after* peephole
  (`emit → peephole → measure → assign PCs + scratch → lower to bytes`), which folds the
  code-relative scratch two-pass in cleanly (code length is invariant to the scratch *value*).
  Unlocks peephole, a small *what's-in-`HL`/`DE`* tracker, and instruction-level measurement —
  and it's the shared substrate for the 8-bit path and a future signed-compare, so it pays off 3×.
- **Peephole rules, ranked by pattern frequency.** `Var⊕Var` / `Var⊕Lit` for commutative ops →
  drop the `PUSH HL` / `POP DE` and use `LD HL,(a); LD DE,(b); ADD HL,DE` (`ED 5B` / `11`) — the
  biggest cumulative win, because add is everywhere; store-then-reload elision; redundant
  `LD H,0` / `EX DE,HL` pairs; dead push/pop around leaf operands. Each rule's correctness
  predicate is **effect-free leaf operands** (the PUSH/POP scheme is evaluation-order-safe; the
  flat form isn't if an operand can `poke` / call).
- **8-bit path (follow-on, behind the seam).** `u8` ops compute in 16-bit `HL` + mask today;
  computing in `A` is a size win for chuk-speccy byte code and pairs with the `[u8; N]` field ask.
  Wants the seam first so peephole can clean the H/L splits at the boundaries.
- **DoD — measure-first, double-tested.** Snapshot the library's per-fn byte sizes via
  `Program::size_report()` *before* the seam (proves the prize is real; the golden to beat), and
  *count* `Var⊕Var` vs general binop sites rather than assuming the ranking. Every peephole rule
  ships **two** tests: a `tests/diff.rs` case (behaviour unchanged — the rustc-vs-emulator net)
  **and** a size/shape assertion (a no-op rule passes diff trivially, so prove it fired).

**Wider state — `u32` in state, then fixed-point (kill the overflow footgun).** `u32` already
exists as a *local* (`Width::DWord`, `gen_expr32` carry-chain in `HL:DE`); the gap is persisting
it in **state**, which removes the `u16` ceiling that pushes `euclid_sq` / `weighted_sum` /
running-stat cells toward "ask the agent to write Python."

- **`u32` in state — the work is the typed-state ABI, not the codegen.** The carry-chain
  arithmetic exists (`ADD HL,DE` then `ADC HL,DE`); the codegen gap is a `u32` *field* load/store
  (two slots) + `layout.rs` giving the field two slots. The real work is the ABI surface the
  discovery/graph layer rides on: `state_addrs` must carry **width** (a `u32` field is 4 bytes /
  two slots), `CellHost::run_state` / `read_named` → PyO3 → MCP `cell_run(fields=)` learn to
  read/write a 4-byte little-endian field by name, and **CellGraph** type-checks / routes `u32`
  edges — an ABI-version-bump candidate. Per-op fork mirrors today: **carry-chain in software for
  add/sub/shift** (authentic on *both* targets — no trap), **host trap for mul/div** (software is
  disproportionate); only mul/div needs a Spectrum software sibling.
- **Fixed-point — a convention on `u32`, not a type.** Q-format is a `u32` with a point
  convention: `weighted_sum` with a Q8.8 weight is `(a*w) >> 8` — a `u32` multiply + a shift the
  compiler already has. So it's a library convention + helper cells (`q_mul` / `q_div`) + an
  optional manifest **scale** annotation, riding on `u32` — *not* a new type the agent must learn.
- **Determinism split (write it down before the first wide op).** Integer / fixed-point /
  **softfloat** are all deterministic (softfloat is pure-integer IEEE-754, bit-identical
  cross-arch) → **in scope, gated on size/cost** (`max_code`, `trapped_ops`); the determinism
  clause is **never waived**. **Hardware float + transcendentals** are non-deterministic or heavy
  → **out**, the Wasm lane the non-goals reserve. `u64` on demonstrated eval need.
- **Cost honesty + DoD.** Wide / trapped ops are **counted in `trapped_ops` and gated** (capped,
  halted on budget), never folded into a cycles reward (extends the gate-not-gradient rule). The
  DoD grows one column: `Cell-target trap ≡ Spectrum-target software ≡ rustc`, under the
  fast-vs-authentic + cross-arch determinism fuzz. Size the prize first: **audit which library
  cells are *accidentally* capped by a `u16` intermediate** (`euclid_sq`, accumulators) vs
  *deliberately* saturating (`add_sat` / `mul_sat` — correct as-is).

### `rustz80` frontend — features the chuk-speccy authoring-plane kit needs

A second, *frontend*-side ask on the shared compiler, driven by chuk-speccy's authoring plane
(its spec 08, track E): **the subset must widen so composable SDK kit types and assets compile
*pure*** — one `impl Game` source → a host build **and** a bootable tape. The pure-Snake seam is
closed today (`chuk-speccy-sdk/samples/snake_game.rs` compiles both ways and boots on the real
ROM), but only inside a narrow envelope. Each item is a concrete blocker found while shaping the
SDK kit; the `file:line` is `rustz80` 0.5.0. These sit **below the agent-tool arc above** (the
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

**Codegen robustness & size — the *other* kit blocker (not the type frontend).** Building real
games on the SDK (`chuk-speccy-sdk/samples/platform.rs` + `chase.rs`) surfaced the codegen-side
limits on how big/clean a game can be. *Verified against rustz80 **0.6.0** on 2026-06-28 —
chuk-speccy is now on 0.6:*

- **Dead-code elimination — emit only reachable functions. ✓ (generic) + extended to the game
  path.** `compile_to_tap` / `compile_file_pruned` DCE rooted at the entry. But the **frame-loop
  entry (`codegen_loop`) did *not* prune** — the SDK compiles games through it, so on 0.6 a game
  still got the *whole* prelude until `dce::prune(funcs, &[entry])` was added to `codegen_loop`
  (done, pending review). Effect (verified on the real ROM): `platform` now shows a numeric
  ROM-font score (`__frame_number`), and `chase` — which doesn't call it — is unaffected (the
  routine is pruned from its tape). Keystone unblock for shared prelude routines (numeric/text HUD,
  sprite-blit, attribute-collision, screen-addr LUT).
- **Function inlining — so clean decomposed code is as compact as hand-inlining. ✓ done
  (`src/inline.rs`), with argument substitution + slot reuse.** Folds each **single-call-site**,
  early-return-free, scalar/void function into its one caller (then DCE drops the now-dead def).
  Single-call ⇒ never duplicates code (pure win: drops the call/prologue/epilogue + param copies).
  Two refinements make "as compact as hand-inlining" *literal*: **(a) argument substitution** — a
  *pure* (`Var`/`Lit`/`&local`), *read-only* param is substituted straight into the body, so there's
  no param-bind `Assign` and no param slot (a `&mut self` method with var/const args inlines to
  exactly the hand-written body — zero overhead); **(b) slot reuse** — callee locals sit at a `water`
  mark that pops after each inlined body, so siblings reuse slots and the scratch region grows by the
  *deepest* inline, not the *sum*. Runs before DCE in `codegen_loop`, `compile_file`,
  `compile_file_pruned`, `compile_to_tap`. Diff-validated against rustc by six behavioural tests in
  `tests/diff.rs` (scalar `&mut self`, scalar assign, array-field write in a loop, two siblings
  sharing slots, a nested kept call, and a helper-then-movement multi-array-field program).
  **Result: `chuk-speccy`'s `chase` now decomposes into clean `step_enemy`/`caught` methods and the
  tape is *smaller* than hand-inlined (4259 vs 4446 bytes).** *Follow-ups (nice-to-have):* inline
  calls in expression/condition position (hoist to a temp) and tuple-return calls (today only `f(a);`
  / `x = f(a);` statement positions inline); small multi-call inlining; the Stage-2
  peephole/strength-reduce pass.
- **Code↔scratch collision — the real `chase` "ceiling". ✓ fixed (dynamic scratch placement).**
  Locals lived at a *fixed* `SCRATCH = 0x9000`, but code grows up from `ORG = 0x8000` — only 4 KB of
  code space. `chase` (~4.3 KB code) overran 0x9000, so the per-frame slot writes silently corrupted
  the overrun machine code → "frozen" enemies. (This is what masqueraded as an inliner bug: inlining
  shifted the layout so the corruption became fatal; the inliner's IR was correct all along — proven
  by the diff tests + an IR dump.) Fix: `codegen_loop` now places the scratch region **just above the
  emitted code** (a measure-then-place two-pass; code length is invariant to the scratch *value*),
  with `state_base` as the ceiling. `codegen_program` keeps the 0x9000 default (cells unaffected).
- **Error on over-budget — never silently miscompile. ✓ (game path).** `codegen_loop` now asserts
  that `code_end + locals*2 <= state_base` and panics with a diagnostic rather than emitting a wrong
  tape. *Follow-up:* return a `Result` instead of panicking, and extend the guard to `codegen_program`.

*Already in 0.5/0.6 (the SDK is now on 0.6):* `&mut self` methods that mutate through the receiver,
`+=`, `for` ranges, local `[u8; N]`, structs/enums/methods. (Re-verify the *frontend* asks above —
nested struct fields, signed/`u32` fields, `[u8; N]` *fields*, `&CONST→addr` — against 0.6; some may
already be done, e.g. a `bitmap` example suggests const-data progressed.)

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
