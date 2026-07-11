# cell80 — Roadmap

> **Execution plan:** [roadmap-phases.md](roadmap-phases.md) sequences the work as
> phased gates (0: determinism contract ✓ → 1: LLM-facing compiler → 2: retrieval →
> 3: trust → 4: codegen stage 2 → 5: multi-target, the cell-family —
> [13-multi-target-spec.md](13-multi-target-spec.md) → 6: model-native cells —
> [14-model-native-cells-spec.md](14-model-native-cells-spec.md)), with a DoD per
> item and the end-state narrative. This file stays the ledger of what's *built*.

cell80 is the deterministic **executable-tool-capsule** layer extracted from
[`chuk-speccy`](https://github.com/chrishayuk/chuk-speccy): a Z80 CPU core (`z80`), a
restricted-Rust → Z80 compiler (`rustz80`), and the cell micro-VM + tooling. The north star:

> Agents discover, inspect, compose, and run **millions of tiny executable tools** without
> loading their schemas into context. Each tool is a self-describing `.cell` cartridge.

## Built

**Compiler (`rustz80`).** `syn` frontend → typed IR → Z80 codegen through the symbolic
instruction layer (`Ins` — labels/calls/slots resolve at one final encode pass) with a
measured fixpoint **peephole** (−4.3 % corpus code size; HL accumulator, RAM scratch
register file, ORG 0x8000). Subset: `u8`/`u16`/`u32`, arithmetic, `if`/`while`/
`for`/`loop`, early return, arrays, `struct`/`enum`, functions, methods, `poke`/`peek`/`inport`.
**Booleans:** comparisons (`< <= > >= == !=`) work both as branch conditions *and* as `0`/`1`
**values** (`(a < b) as u16`), short-circuit `&&` / `||`, and bit shifts take a **runtime**
amount (`x << bit`) — so predicates and bit ops are one-liners. **Dead-code elimination**
(`compile_file_pruned`) keeps only functions reachable from the roots — the cell layer uses it
to prepend a shared-kernel prelude and prune whatever a cell doesn't call. The dialect is *also
real Rust* → every program is differential-tested against `rustc` on the emulator
(`tests/diff.rs`).

**The cell-family (Phase 5 — multi-target, in flight; spec
[13-multi-target-spec.md](13-multi-target-spec.md)).** WS-A shipped 2026-07-10/11:
the **target descriptor** (Spectrum48/Cell as instances; codegen forks on the
descriptor, never the target), the **reference IR interpreter** (a standing oracle in
the diff battery beside rustc and both Z80 targets), **evaluation order canonicalized
left-to-right wherever observable** (the one sanctioned golden break: 4/347 programs,
−8 bytes), the **explicit width-bridge family** (`SignExtend`; `i16 as u32` unfrozen),
**i32 through the IR** (interpreter-accepted; signed-32 gated out of Z80 codegen with
an instructive pointer at the backends that have it), and the **`cell80-core`
extraction** (typed IR + passes + interpreter + descriptors, dependency-free; rustz80
is backend zero on top). WS-B's compiler shipped (B1–B3): **`rustrv32`** — the RV32
`Ins` sibling + exact encoder (encoding goldens), the cycle-accounted RV32IM executor
(RISC-V-exact M semantics, Hazard3-truth misalignment faults, provisional cycle table
pinned by test), and full codegen over the shared IR (family 2-byte slot ABI in a
64 KiB window mirroring the interpreter's map; native signed-32). The battery runs
every program on **five systems** — rustc, 2× Z80, the interpreter, RV32 — and
`run_to_memory` compares the RV32 window against the interpreter image **byte for
byte, unmasked**. Per-file coverage ≥90% across both new crates. Demo:
`cargo run -p rustz80 --example cell_family` (gcd: 6021 on all three bodies — 12,882
authentic T-states vs 490 provisional RV32 cycles; an i32 deadband runs natively on
RV32 while backend zero refuses, as pre-registered). Since then: the
**determinism-fuzz battery** (seeded random width-lattice programs across the whole
matrix + the exact-cycle/window fingerprint); the **GNU-gas emission adversary**
(binutils independently re-encodes every instruction shape, CI-required on linux —
the encoder is never self-refereed; teeth proven by deliberate corruption);
**`.cell` v10** (WS-E1 — the manifest's target id names the machine body and a host
refuses one it can't run; the family hash, SHA-256 over canonical source, is what
"same cell, N bodies" means formally); and **WS-E2 resolved for free** — the family
slot ABI + the shared window map make `state_addrs` target-portable as-is. The
descriptor story sharpened into three layers (spec §2.1a: ISA backend / core timing
model / platform; a certified target is the named triple). Owed: WS-E3's per-body
host, the Sail/spike execution adversary, the RV32 peephole, the B4 `mcycle`
co-sign on silicon.

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
- **The LLM-facing compiler (Phase 1 of [roadmap-phases.md](roadmap-phases.md)).**
  **`if`/`match` as expressions** (`let x = if c { a } else { b };` — let/assign/return/
  tail positions, nesting, `else if`, u32 arms); **instructive diagnostics** — every syn
  `{:?}` dump replaced with prose naming the construct and the accepted rewrite, and
  `compile_fn` on multi-item input points at `compile_program`; **signed `i16`**
  (two's-complement: signed compare via S ⊕ V, truncating `__sdivmod16` through the
  per-target unsigned core, arithmetic `>>`, sign-boundary diff tests) with the
  fixed-point idiom documented; and the **repair-rate eval** (`cell-eval repair`) that
  makes the diagnostics *measurable* — one shot, compile **and** reproduce the intended
  I/O to count. Baselines: repair@1 **0.60** (granite4.1:3b) / **0.90** (gemma-4-26B).
- **Determinism contract closed (Phase 0 of [roadmap-phases.md](roadmap-phases.md)).**
  **Recursion is rejected at compile time** (a call-graph cycle check at lowering — Stage-1
  static locals made the slot-after-call factorial silently return 1; now it's a named-cycle
  error, probed by reject-tests). **Both targets sit under the rustc oracle** — the diff
  harness compiles every test for `Spectrum48` *and* `Cell`, services the full trap set, and
  asserts cross-target agreement (the trap path the VM ships had never been diff-tested).
  **`/ 0` is a typed halt** (`Halt::DivByZero`; `CellConfig::div_by_zero` — `Halt` default,
  `Saturate` legacy opt-in, carried in the image; Spectrum keeps saturation, documented).
  The dialect's guarantees are written down in
  [10-dialect-semantics.md](10-dialect-semantics.md).
- **Z80 core conformance** — the foundation everything runs on passes the per-opcode
  **SingleStepTests** vectors **1,530,000/1,530,000** (initial→final state + cycle counts for
  every opcode/prefix incl. undocumented: base/CB/ED/DD/FD/DDCB/FDCB) and the **ZEXDOC**
  exerciser ROM; both fetch-on-demand (`z80-tests/`, see its README). Building this caught
  six real core bugs — the EI/IFF timing model, the undocumented repeat-flag rules for the
  LDIR/CPIR/INIR families, the DD/FD-prefixed SCF/CCF Q-latch, and `LD (IX+d),n` timing —
  now fixed.
- **Standard library** — `cell80/cells/` is now **96 cells** (100 minus four cells the
  Phase 2.2 admission gate found were exact behavioural duplicates — `argmin2`/`argmax2`/
  `quantize`/`wrap`, folded into `is_gt`/`is_lt`/`safe_div`/`safe_mod` as aliases; see
  `docs/library-growth.md`): the 8 originals plus ~12
  confusable families — **predicates**, **safe arithmetic**, **bounds**, **percent/ratio**,
  **ranking/stats**, **bit/mask**, **number theory** (`lcm`, `is_prime`, `isqrt`,
  `factor_count`, `pow_mod`, …), **distance** (`chebyshev`, `euclid_sq` — state-cell siblings of
  `manhattan`), **bit/encoding** (`rotl16`, `reverse_bits`, `bit_length`, `swap_bytes`, …),
  **hashing** (`hash_pair`, `fnv1a_step`, `crc8_step`, `mix16`), **bucketing/conversion**,
  and the **wide (u32-in-state) siblings** (`square_wide`, `weighted_sum_wide`; `euclid_sq`
  carries a wide `dist` field). All indexed + searchable, with a per-cell host-oracle
  (`cell80/tests/library.rs`) and direct/paraphrase/adversarial retrieval rows.
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
3. **Type-led / behavioural discovery — wired into the live path (2026-07-05); the lever is
   behaviour, not text. ✓ shipped.**
   The bet was that a cell's typed signature could out-rank text for paraphrases. Built,
   measured, and **now live**: `TypeLedIndex` (`cell80/src/typeled.rs`) powers
   `CellHost::search` — the CLI, `cell80-py`'s `search`, and `cell80-mcp`'s `cell_search` all
   route through it (`CellHost::search_scored` deliberately stays plain TF-IDF; its raw cosine
   feeds `cell-eval`'s calibrated tiered-retrieval margin, which must never be rescaled). A
   **type-led** re-rank by behavioural **predicate-ness** — learned from the corpus as
   smoothed log-odds, *no hardcoded vocabulary*, labels free from running the cell — measures
   **identically tied** with plain TF-IDF on the current 209-cell library (82% / 36% / 50%
   direct/paraphrase/adversarial P@1) — not a regression, but not the paraphrase fix either.
   The honest reason: the residual misses are *same-shape siblings* (`min`/`min_u32`, the
   five-member sign-magnitude family `smag_add`/`sub`/`mul`/`div`/`cmp`) that a
   predicate/transformer axis can't separate — both sides of each pair are non-predicates, so
   there's no disagreement to re-rank on. What *does* separate same-shape siblings is
   **behaviour**: on `(3,7)→3` only `min` matches, not `max`. So **behavioural I/O-example
   routing** is first-class — `CellHost::route_by_examples`, a `route` serve verb, and the
   **`cell_route_by_example`** MCP tool over `rank_by_examples` (`cell80/src/fingerprint.rs`):
   phrasing- and language-independent selection grounded in what the cell *does*. **Next** — richer
   behavioural fingerprints (output cardinality, monotonicity) and discriminating-probe selection,
   then let the model *learn* to pick probes (where SOMA would schedule, not own).
   **Scoped before building — narrower payoff than it first looks.** Cardinality is cheap (reuses
   `is_predicate`'s probe-and-count pattern) but only discriminates *within* a family — it
   separates `smag_cmp` (cardinality 3) / `smag_eq` (cardinality 2, passes the predicate test) /
   `smag_add` (full-domain cardinality) from each other, but `min`/`min_u32` are both monotonic
   *and* both full-cardinality, so neither axis touches the headline same-shape-sibling case two
   sentences up. Monotonicity isn't a drop-in extension of the flat `PRED_PROBES` scheme: it needs
   a real per-argument sweep (hold every other arg fixed, vary one), and three things break a naive
   version of that — state cells' field-cycling doesn't map onto "hold other args fixed," `u32`-
   widened cells only get a sliver of their domain from `u16`-scale probes, and cells with validity
   `limits` halt/escalate outside their valid range, turning a sweep into gaps instead of a
   monotonic run. Net: worth building for the `smag_*` family's internal collisions specifically;
   not a fix for `min`/`min_u32` — that one still wants behavioural routing (above) or a genuine
   arity/structural-shape axis on `TypeLedIndex`, neither of which cardinality/monotonicity supply.
   **Fixed (2026-07-05): `Fingerprint::compute` now digests the full tuple-return surface.**
   The stateless branch used to digest only the primary (`HL`) register (`Some(r.result)` when
   `state.is_empty()`), never `DE`/`BC` — unlike the state-cell branch below it, which already
   digests every named output field via `digest_state`. A free function returning a
   `(u16, u16, u16)` tuple was therefore fingerprinted as if it only returned its first element:
   the admission gate refused a scoped `sort3` cell (`(min, mid, max)` in one call) as a duplicate
   of `min3` at 1.00 agreement, correct by the fingerprint's own math (`min3`'s single output *is*
   `sort3`'s first tuple slot) but blind to `sort3`'s real payload — `mid`/`max` living in the two
   registers it never looked at. The fix stays inside the same function: `ret_reg_count` reads the
   entry's declared return type (a tuple's element count from `signature.ret`, else `1`), and
   `digest_regs` folds `r.regs[0..n]` position-sensitively. **A scalar declares `n == 1`, so the
   digest is exactly `regs[0]` — every existing single-value fingerprint is byte-identical** (no
   admission-gate shift; the 70-test `cell.rs` gate + the 8 prior fingerprint tests stay green),
   and no tuple-returning free fn is currently in the library so nothing live moved. Unit-tested
   (`ret_reg_count` cases + scalar-identity) plus the behavioural `sort3`-vs-`min3` separation.
   **Residual (out of scope):** a `u32` return still digests only its low word (`HL`); noted in
   `ret_reg_count`'s doc as a separate, no-live-cell gap.
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
6. **Grow the standard cell library — 653 cells across 30+ families (2026-07-11).**
   `cell80/cells/`: predicates, safe arithmetic, bounds, percent, ranking/stats, bit/mask,
   number theory, distance, encoding, hashing, bucketing/conversion, packing/BCD, vector,
   scoring/choice, agentic-runtime, running-stats, spatial/grid, stateful/RNG, signed-deltas —
   plus the **GSM8K math campaign** (`docs/math-campaign-spec.md`, M1): checked/exact wide
   arithmetic, fractions, money/basis-points, unit-dimension codes, verifier/ranker, and the
   sign-magnitude family (`smag_add/sub/mul/div/cmp/is_nonneg/eq`) — each with retrieval rows +
   a host-oracle test, **modular** via the shared kernel prelude + DCE (`docs/
   library-growth.md`). Math-campaign hand-authoring is now **paused on purpose**: further
   growth is meant to come from **precipitation** (item 9, `cell_solve`) — real problems
   surfacing which schemas actually recur — not more speculative candidates. Every batch has
   cost real, only partially-recovered retrieval precision; the library-growth doc tracks the
   tradeoff checkpoint by checkpoint. **310→313 (2026-07-10) is the ecosystem-mining track
   instead**, sanctioned as a cheaper-than-authoring source distinct from speculative
   hand-authoring: `linear_solve_1var`/`linear_eq_holds`/`difficulty_zone_step`, ported from
   `chuk-math-gym`'s linear-equation and curriculum-scheduling domains after confirming
   `chuk-mcp-math` was already fully mined and `chuk-synthetic-data` negligible
   (`docs/library-growth.md`'s "Mine the ecosystem first"). **313→395 (2026-07-11) is the
   first library batch run through the `Workflow` tool**: `chuk-math-gym`'s remaining
   `arithmetic/` domain (closing out ecosystem mining) plus **systematic family
   expansion** — 8 discovery agents fanned out over pack clusters, finding genuine gaps in
   existing patterns (missing width/sign/arity siblings, a predicate with no complement)
   rather than inventing speculative candidates, pulling first from library-growth.md's own
   "Next waves" backlog. 104 raw candidates → 90 deduped → 90/90 individually
   authored-and-verified (0 failures) → the real admission gate then caught 8 behavioural
   duplicates no single candidate's own verify step could see (checked only against the
   pre-batch library) → **82 landed, 395 admitted / 0 refused**. Independently re-verified
   after the fact (gate re-run, a cell-quality spot-check, the codegen-golden diff
   confirmed purely additive) rather than accepted on the workflow's own report — full
   account, including the admission-gate's duplicate-pair-naming quirk this batch surfaced,
   in `docs/library-growth.md`'s "Systematic family expansion" section. That same pass also
   closed two long-open backlog items by hand (not delegated, since the numerics needed real
   reasoning): **`cosine_score_approx`** (vector) — blocked for many checkpoints on
   "sqrt(norm_a\*norm_b) without overflow," solved once `isqrt_u32` (a wide integer sqrt this
   batch itself added) made the trick work: two u16-bounded norms always fit a u32 product,
   no overflow ever possible — and **`lerp_i16`** (signed-deltas) — `q_lerp`'s signed sibling,
   blocked on `b-a` exceeding `i16`'s own range even for valid `i16` endpoints, solved with
   the sign-magnitude pattern this session proved out (395→397). Q16.16 fixed-point plumbing,
   the third named backlog item, was checked and confirmed still genuinely blocked (needs a
   64-bit intermediate the dialect lacks — real compiler work, on WS-C's own roadmap, not
   forced here). **397→500 (2026-07-11) is round 2 of the same Workflow approach**, 13
   narrower single/dual-pack clusters (round 1's broad clusters had already taken the easy
   wins) digging deeper: 126 raw → 111 deduped → 110 verified → gate caught 7 duplicates →
   **103 landed, 500 admitted / 0 refused**. The dedupe step stalled repeatedly on the first
   attempt (one agent choking on 126 candidates); fixed by splitting it into two lighter
   passes and resuming from the cached discovery results rather than losing that work.
   Deliberately excludes a concurrent session's in-flight cartridge-format (v10) changes
   sitting uncommitted in the same working tree — not this batch's to own. The retrieval
   kill-gate was re-checked at both 395 and 500 cells (checkpoints 17-18,
   `cell-eval/baselines/library-scale-curve.json`): paraphrase is flat and **adversarial is
   now measurably above the original 114-cell baseline** despite the library growing 4.4×,
   so growth remains sanctioned. **500→653 (2026-07-11) is round 3, one discovery agent per
   single pack** (32 packs) — deepest yet, and it landed the most cells of the three rounds
   (153, confirming round 2's own finding that narrower digging finds more): 197 raw → 160
   deduped → 159 verified → gate caught 6 duplicates → **153 landed, 653 admitted / 0
   refused**. **This is where the kill-gate finally tripped for real (checkpoint 19)** —
   paraphrase fell to 0.3736, a 5.1-point drop from checkpoint 1's baseline, over double the
   ~2.3-point drop that triggered the original checkpoint-10 pause-and-fix cycle. Flagged to
   the user rather than launching a round 4 past it. Diagnosis: of 386 cells appearing as a
   miss, only 11 were genuinely under-tagged — the other 375 are the same-shape-sibling
   saturation this project has repeatedly diagnosed as not fixable by wording, and three
   rounds of deliberately building missing siblings is exactly what grows that class. Fixed
   the 11 real gaps, verified each fix landed on its target query — **checkpoint 20**:
   paraphrase recovered to 0.3866 (~25% of the drop), adversarial to 0.5000 (+8.3pt). A
   partial recovery, same honest shape as checkpoint 11 — the dominant remaining cause needs
   the structural lever this project has already named and not yet built (behavioural
   I/O-example routing, or a type-led index that discriminates on structural shape). Full
   account, including a second and larger instance of shared-checkout friction (a concurrent
   `Cartridge::program → Cartridge::body` refactor spanning 7 files, versus round 2's single
   `tfidf.rs`), in `docs/library-growth.md`'s "Round 3" section.
   **The 209→310 gap is `docs/math-server-map.md`'s mining pass** (`chuk-mcp-math-server`'s
   642 functions classified against the live library) **plus its full harvest, waves 6-14
   (2026-07-07 to 2026-07-09)**: number theory (Möbius/omega/divisor-power-sum/Jordan
   totient/Carmichael lambda, figurate numbers, recursive sequences, digit operations,
   modular/classic number theory, combinatorics), the geometry/vector integer subset
   (3D distance, cross/triple products), the matrix "vector floor" exception
   (`matrix_det_2x2`/`matrix_solve_2x2`), and bivariate statistics from precomputed sums
   (`covariance`/`linear_regression_slope`/`correlation`/`effect_size_r`, the last two Q8.8
   via the same scale-before-sqrt precision technique `q_sqrt` uses). Unlike the paused
   speculative M1 growth, this was **catalog-driven, not speculative** — every candidate
   traced to a real function in an external math library, each folding duplicates into
   generalizations rather than authoring near-identical siblings (`polygonal_number(5,n)`
   *is* `pentagonal_number`, `lucas_u_v(2,1,n)` *is* `pell_number`/`pell_lucas_number`).
   **Wave 14 (`correlation`, `effect_size_r`) closes the original 77-candidate list in
   full** — every `candidate`-classified function that map named is now landed, folded, or
   explicitly deferred with a documented reason. Full per-wave detail:
   `docs/library-growth.md`'s wave notes; live status trail: `docs/math-server-map.md`'s
   "Update" bullets.
7. **Signed `i16` — ✓ done (Phase 1.4; see "Built" above).** Signed compare via S ⊕ V,
   truncating `__sdivmod16`, arithmetic `>>`; unblocks scoring/delta cells (`x_y_delta`,
   signed `lerp`, risk deltas) — the library's signed wave can now land.
8. **Experimental: outcome-specified synthesis** (`cell80/src/synth.rs`) — the *inverse* of
   `CellGraph`: given input→output **examples**, search over short cell chains and verify
   candidates by execution (the verifier is the engine). A deliberately separate mode from
   normal `search → inspect → run` tool-calling — *given behaviour, discover a graph* — kept out
   of the main pitch. Honestly gated: a learned value heuristic only *ties* the hand Hamming
   heuristic at equal budget so far (a kill gate, not a given). See `examples/composition_eval`.
9. **`cell_solve` — the math campaign's M2, ✓ shipped (2026-07-05).** The plan IR
   (`cell80/src/plan.rs`) is a wire format between model and host, never executable: a model
   extracts typed, unit-tagged quantities + an op chain (`add`/`sub`/`mul`/`div`) + a target,
   the renderer emits canonical dialect Rust (deterministic — same schema, same source, same
   artifact hash), and it compiles and runs **as a cell**. `CellHost::solve` renders → compiles
   → runs each candidate (memoized), kills plans that escalate or halt (named reason, never a
   wrong answer), and — when more than one plan survives — **always** perturbs every quantity
   by +1 and keeps the largest self-consistent group, agreement or not (a fix landed after
   test-driving found the original short-circuit would've accepted a coincidental agreement,
   the same failure class as the documented `min`/`median3` register-0 coincidence, without
   ever stress-testing it). Surfaced everywhere: CLI `cell80 solve`, `cell80-py`'s `solve()`,
   MCP's `cell_solve`. **A 127-row, unfiltered, consecutive slice of the real GSM-8K test
   set — not hand-crafted, not cherry-picked — solved 123/123 (4 skipped as genuinely
   unrepresentable, ~97% representability)** through it (`cell80/examples/
   m3_gsm8k_smoketest.rs`), real evidence beyond synthetic cases that the loop answers
   correctly end to end at meaningful scale. The newest 50-row batch (rows 78-127) hit zero
   unrepresentable problems — the 4 known gaps are real but not frequent, not outgrown.
   Findings from that smoke test: the plan IR has
   no comparison/decision opcode at all (a "pick whichever is bigger" problem can't be
   rendered — not yet acted on); fractional-dollar amounts need a firm cents-always
   convention (found, then **fixed** — every money quantity in the smoke test now rescales
   to cents); the identifier blocklist didn't cover Rust's full reserved-keyword set (`final`
   slipped through to a raw `rustc` error instead of a clean one — found and **fixed**,
   regression-tested); and genuine extraction ambiguity (not arithmetic error) is a real
   fragility source even for a careful extractor (not fixable — a property of the English,
   not the renderer). Full spec, sequencing gates (M0-M4), and the pre-registered hypotheses:
   `docs/math-campaign-spec.md`. The extraction leg of that spec is now **superseded by
   the amended registration** (`docs/math-campaign-amendment.md`) — see item 10.
10. **M2.5 canonicalization + M2.6 fold/width/typed-diagnostics + M2.9 `cell80 compose`,
   ✓ shipped (2026-07-06).** PlanFix's findings (`experiments/planfix/`) demoted plan-IR
   JSON to an internal wire format and moved every deterministic repair into the
   compiler: `rustz80::canon` (text→text before hashing — the artifact hash covers a
   raw-text source hash, so AST-only canonicalization would leave precipitation
   unmeasurable) alpha-renames to `q*/v*` slots in dataflow order, topo-sorts ops,
   folds constants exactly (decimals become exact fractions; inexact constant division
   is a typed compile error), defers division, and auto-widens the lane when constants
   exceed `u16::MAX`. `rustz80::diag` gives every pass stable `E*` codes with
   `suggested_fix`; the dialect normalizer is a code→rewrite table, not string
   matching. The plan renderer emits slots natively (the `final`-keyword class is
   impossible by construction; nouns survive only as rename metadata), all **seven
   registered M2.5 acceptance tests are green** (`cell80/tests/canon_acceptance.rs`),
   and Light mode is byte-stable across the whole library (codegen golden unchanged).
   `cell80 compose <dir> <src.rs>…` productionizes the planfix link loop: Full canon →
   `E0504`-cued resolution (search + arity behind a measured 0.6 confidence floor —
   a nonsense call name is a typed refusal, never a silent guess) → the registered
   N-derivation agreement gate (unanimous / majority-flagged / escalate) → `--facts`
   provenance. **First model-run through the pipeline (M2.8 item 1,
   `experiments/planfix/crosscheck-m26-findings.md`): the registered gemma4 yield
   prediction FAILED (75% vs ≥90% predicted) and is banked as a miss; precision — the
   revert trigger — held at 100% across all three models (23 accepts, 0 wrong).**
   4 of gemma's 5 escalations are one-path-correct and would accept under the 2-of-3
   rule → **next: M2.7** (third derivation, decorrelated reader), then the error
   chase (qwen's `expected ';'` dialect, granite's `E0502` mass) from captured
   sources, then M2.8 proper and M3.
   **M2 phase CLOSED (2026-07-07).** Everything between the amended registration and
   M3 is done, measured, and merged. The gate stack in its final pre-campaign form:
   Full canonicalization (slots · topo order · exact folding · defer-division ·
   if-value with lazy arms · casts · SSA reassignment · literal lifting · **checked
   emission** — overflow/negatives escalate, never wrap) → the link loop (exact-id
   first, then confidence-floored search+arity; wide `_u32` kernels in the prelude)
   → the registered N-derivation gate (`unanimous`/`majority`-flagged/`escalate`/
   `degenerate_zero`/`battery_escalate`) → counterfactual battery certificates →
   facts + cost (`cycles`/`trapped_ops`) per derivation. **Five registered
   amendments** (zero-guard · `E0205` method→kernel · `E0207` verify-rewrite ·
   `E0208` advisory suffixes · `E0209` checked-lane narrowing-drop), each
   replay-verified at 0 accepted-wrong. **M2.8 complete**: granite/gemma/qwen
   pipeline runs, PAL baseline (H-M2 passes for gemma: cells 100% vs PAL 90%+2
   silent), battery-on-composed (caught a real exact-division-coincidence agreement
   in the wild), cross-language parity 7/7 (which found and fixed lifting's
   silent-wrap hole). Slice scoreboard: **gemma 20/20 · granite 45% solo / 75%
   ensemble · qwen 15% solo / 65% ensemble — all at 100% precision; zero wrong
   accepts across every configuration and two rounds of coverage expansion.** The
   residual failure set contains nothing the compiler can honestly fix (stated
   answers, genuine comprehension splits, tally-gated dialect one-offs). **Next:
   user sign-off on the M3 registration update (drafted in
   `docs/math-campaign-amendment.md` — ensemble column, H-P2 kill applied,
   battery certificates mandatory, frozen E-codes), then the campaign runner
   (checkpoint/resume · per-generation provenance: model digest/seed/options ·
   facts + cost capture), then N=1,319.**
      **Error-chase backlog, priced from captured sources (2026-07-07; (a)(b)(c)
   ✓ landed same day — see `experiments/planfix-m2-findings.md` §4c):**
   (a) **casts in the straight-line subset** — `as u16`/`as u32` currently soft-fail
   canon Full, which blocked the `E0205` method rule and lifting on granite row22's
   otherwise-clean derivation; small build, unblocks two landed passes on real
   sources. (b) **if-value (select) support in Full canon** — the *inline* arm, the
   most common derivation shape (19/20 correct on gemma), gets Light fallback today:
   no lifting, no folding, no battery participation; supporting `if c { a } else
   { b }` as a canonical select node is the single largest coverage item and also
   reaches the verify-`if` shapes granite wraps its arithmetic in (row89-class).
   (c) **verify-`if` → computed-side rewrite** (`if E == lit { lit } else { 0 }`
   returns `E`) — converts granite's signature verify-not-compute dialect into
   honest computation; output-changing, so it needs its own registration plus a
   captured-source precision recheck before landing. (d) **free-fn `_u32` siblings**
   — the widened lane can only link free fns and every wide cell is a state cell;
   legal since two-u32-params, and the linker already prefers `{name}_u32`.
   (e) **`then`-sugar / Model-Rust superset entries** — priced at ~2–3 rows today,
   entry decided by M3 escalation tallies per the dialect discipline, and only safe
   behind the (landed) zero-guard. Explicitly *not* backlog: qwen's
   stated-answer-then-work (no honest repair exists — `correct_via_solve` forbids
   it; its path is the ensemble reader or a registered instruction-shape amendment),
   and per-model prompt schemas (registered exclusion). Tooling debt from the same
   chase: `cell80-py` sits outside `cargo test --workspace`, so `CartridgeOpts`
   changes break it invisibly until CI — needs a local-loop parity check. CI re-priced the retrieval direct-p@1 floor
   to 0.79 at 263 cells (measured 0.7934 — wide-sibling IDF dilution; the fix is
   width-aware routing in the type-led index, not tag stuffing;
   `cell-eval/baselines/retrieval-direct-misses-263cells-2026-07-06.txt`).
   Findings write-up for the whole arc:
   `experiments/planfix-m2-findings.md` (gemma4 19/20 @ 100% precision under the
   2-of-3 gate; granite 70% @ 100% with the second-model reader; 49 accepts / 1
   flagged wrong campaign-wide, class diagnosed, zero-cost fix proposed).

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

**Stage 2 — codegen quality.** The blocker *was* structural, not effort: codegen emitted raw
bytes straight into a `Vec<u8>` (`a.byte(0xE5)`), so instruction boundaries were gone and there
was no seam for a quality pass. **The seam and the first peephole shipped** (roadmap-phases
4.1/4.2 carry the full dispositions):

- ✓ **Instruction-IR seam (the keystone) — done.** The `Ins` list sits between codegen and
  `finish()` — symbolic operands (labels / slot refs / call targets; the runtime as
  `Ins::Blob`), PC + scratch assignment inverted to a final encode pass exactly as planned
  (`emit → peephole → measure → assign PCs + scratch → lower to bytes`); the frame loop's
  two-pass emission collapsed to one emission + two encodes. Byte-identical to the pre-seam
  compiler over the committed golden corpus (`cell80/tests/codegen_golden.rs` — also the
  permanent regression net: future codegen changes must regenerate it and review the deltas).
  The 8-bit `A` path and signed-compare now have their substrate.
- ✓ **Peephole, first six rules — done, measured.** Site counts over the 100-cell library
  confirmed the predicted ranking (leaf `Var⊕Var`/`Var⊕Lit` pairs 150, store-then-reload 30,
  2-arg call tail 26, literal-add 15, cleanups 4, dead push/pop 2); the leaf-pair rewrite
  landed as `PUSH HL; <leaf>; POP DE → EX DE,HL; <leaf>` (register-state-exact, so it needs no
  consumer analysis — the `LD DE,…` direct form is kept for the commutative-add literal case).
  **−994 bytes (−4.3 %) across 111 of 117 corpus images.** Every rule shipped the two tests the
  DoD demands: a rustc-oracle diff case and a fired-proof shape assertion.
- **8-bit path (follow-on, behind the seam) — first slice ✓.** `u8` ops compute in 16-bit `HL` +
  mask; the win is chuk-speccy byte code (the cell stdlib is 0% `u8` — measured, all u16/u32), so
  this is a games-side size play, not a cell80 one. **Shipped:** width-aware **byte-bitwise high-half
  elision** — a `u8` `&`/`|`/`^` result is always `< 256`, and every byte operand already reaches
  `gen_bitwise` with `H = 0` (literals `< 256`, `Trunc`/`Peek`/byte loads / byte ops all end `LD H,0`),
  so the high-byte half (`LD A,H; OP D; LD H,A`) computed `0 OP 0 = 0` then got masked — pure dead
  work. Skipped for `Width::Byte` (`codegen/expr.rs::gen_bitwise`), −3 bytes per byte bitwise op;
  `showcase/draw` 586 → 583, the whole stdlib byte-identical (golden). Diff-tested vs rustc + a
  fired-proof shape assertion (`tests/diff/bytes.rs`). **✓ Full A-accumulator lane — also shipped.**
  A byte-typed **chain** (`≥2` ops) of `+`/`-`/`&`/`|`/`^` over literal/var operands evaluates in the
  `A` register (`gen_expr8`: `LD A,(slot)` / `ADD A,n` / `ADD A,(HL)` — a new `Ins::LdAMem`), no
  per-op `PUSH`/`POP` spill and no intermediate `LD H,0`, then zero-extends into `HL`. **Gated to a
  chain on purpose:** a *single* byte op breaks even or *loses* (the HL path often already holds the
  left operand; the lane would reload it — measured `a+5` growing 10→12 before the gate), so
  `is_a_chain` requires the left operand to be a byte op too; each op past the first then saves ~3 B.
  Zero cell80 impact (0% u8 → golden byte-identical, no program grew) but **−24 B (−15.5 %) on a
  byte-heavy blend/shade kernel** — the chuk-speccy pixel-code win, real at last. rustc-diff (mixed
  ops, masks, wrapping chain) + fired-proof (`LD A,(nn)` present for a chain, absent for a single op).
  **Still open:** storing the `A` result straight to a byte field (skip the `A→HL→store` round-trip),
  and folding a nested right operand — both smaller follow-ons.
- **`INC HL` strength-reduction (R7) — ✓ shipped, measured.** `LD DE,1; ADD HL,DE` → `INC HL`;
  `LD DE,2; ADD HL,DE` → `INC HL; INC HL` (the `+1`/`+2` value add R2 leaves; 3+1 bytes → 1/2).
  The no-flag-consumer argument is made explicit: the `ADD`'s flags are dead — arithmetic results
  travel in `HL` as values and every comparison recomputes via its own `SBC`, so `INC HL` (no
  flags) leaves the same downstream state; `DE` is scratch. Sized first (67 sites across 190
  cells: 19×`+1`, 48×`+2`) then measured: **−497 bytes across 76 of the golden programs, no
  program grew** (`corpus_rule_ranking` reports `inc_dec: 67`). Ships the two DoD tests — a
  rustc-diff case incl. a `+1`-into-comparison (proving the dropped flags were dead) and a
  loop induction step, plus exact-byte shape assertions (`tests/diff/peephole.rs`,
  `tests/peephole_shape.rs`). The `DEC` counterpart is *not* built: `x - N` lowers to
  `LD HL,N; EX DE,HL; …; OR A; SBC HL,DE` (a different, larger window), and sub-by-constant is
  rarer than the loop-increment `+1`.
- **Window-spanning leaf pairs (R8) — ✓ shipped.** `PUSH HL; <DE-free span ≥2>; POP DE` →
  `EX DE,HL; <span>` (the span reads/writes only `HL`/`BC`/memory — `de_free_span`, so the up-front
  `EX` leaves the same final state with the stack balanced). R7 *creates* this shape: a
  `self.arr[i]` element address is `PUSH HL; LD HL,(self); INC HL…; POP DE` once R7 reduces the
  field-offset add. Sized first (**15 fires across 190 cells**), then measured **−66 bytes across 28
  golden programs, none grew** — bigger than the 15 direct because each `EX DE,HL` cascades into
  R6's `EX;EX` cancellation. rustc-diff (`self.arr[i]` method) + fired-proof counter test; the R6
  cleanup count rose 19 → 22 accordingly.
- **`what's-in-HL/DE` tracker for cross-statement reload elision — still deferred.** Ceiling
  **486** same-slot store→reload pairs, but an *upper bound*: most spans clobber `HL`, so the
  realisable subset is a fraction, and capturing it needs a real dataflow pass (not a window
  matcher) — the biggest, riskiest of the remaining ideas. Deferred until a larger win justifies it.

**Wider state — `u32` in state, then fixed-point (kill the overflow footgun).** `u32` already
exists as a *local* (`Width::DWord`, `gen_expr32` carry-chain in `HL:DE`); the gap is persisting
it in **state**, which removes the `u16` ceiling that pushes `euclid_sq` / `weighted_sum` /
running-stat cells toward "ask the agent to write Python."

- **`u32` arithmetic — ✓ done (the expression lane is complete).** The prize was sized first
  (`cell80/examples/overflow_audit.rs`: **7/7** overflow-prone cells wrong at `u16`), then:
  `as u32` widening (`Expr::Widen`, zero-extend into `HL:DE`), full **`+ - * / %`** — add/sub as
  an inline carry chain (`ADD`/`ADC`, `SBC` — authentic on *both* targets), mul/div/rem via the
  per-op fork exactly as planned: **`ED FE` traps `0x12`/`0x13`** on the Cell target (counted in
  `trapped_ops`) and real **software siblings on Spectrum** (`__mul32` by three 16-bit partial
  products over a new `__mul16w` 16×16→32 core; `__divmod32` restoring division with a 33rd-bit
  forced-commit path, so even divisors ≥ 2³¹ are exact — both emitted through the `Asm` with
  labels, not hand-counted offsets). Mixed-width operands zero-extend (`part as u32 * 100`, the
  unsuffixed-literal mixing rustc allows); `wrapping_*` maps to the mod-2³² ops; `let x: u32 = 5`
  respects the annotation. Divide-by-zero is bounded and target-identical (`q = 0xFFFF_FFFF`,
  `rem = dividend`). Diff-tested against rustc (`tests/diff/u32_ops.rs`, incl. the forced-commit
  divisor and the percent shape) — and every place a `u32` could *leak into a 16-bit context*
  (returns, params, call args, comparisons, conditions, stores, `u32` struct fields) is now a
  **clean lowering error, never a codegen panic or a silent one-slot layout**.
  **Effect on the library:** the five intermediate-overflow cells (`percent`/`permille`/
  `ratio_255`/`scale_percent`/`within_percent`) now compute wide and saturate — the audit reads
  **2/7** wrong, and what remains is precisely *result* overflow (`square(300)`, `weighted_sum`),
  i.e. the u32-in-state prize proper. `u32` **comparisons** stay unbuilt (cells compare via the
  word-split idiom); `Cond32` **landed 2026-07-04**: u32 comparisons in condition and value position (SBC-borrow materialisation, oracle-checked), plus u32 saturating_add/sub riding it.
- **`u32` in state — ✓ done, end to end (ABI v2, `.cell` v4).** The compiler side:
  `layout.rs` gives a `u32` field two little-endian slots (`FieldDef.width` distinguishes it
  from a 2-element array), field access lowers wide (`Var32`/`Assign32` by value,
  new `Deref32`/`Store32` through the `self` pointer), diff-tested against rustc.
  The ABI side, exactly as scoped: `state_addrs` carries a **width per field**
  (`(name, addr, Ty)`; `.cell` format **v4**, back-compat reads of v3/v2),
  `StateCell::set`/`get` and `CellHost::run_state`/`read_named` drive and read a 4-byte field
  by name (PyO3/MCP were already u64-wide — no change needed), and **CellGraph** routes `u32`
  edges — u32→u32 wires type-check, u32→u16 narrowing is rejected *before* running.
  **`ABI_VERSION` bumped to 2** (with the 0x12/0x13 traps): additive on a v2 host, but a v1
  host no-ops unknown trap ids, so the artifact must declare what it needs.
  **Library:** `euclid_sq.dist` is now a wide field (exact 250,000 on (0,0)→(300,400));
  new wide siblings **`square_wide`** / **`weighted_sum_wide`** (100 cells). The
  `overflow_audit` example now ends **3/3 wide fields exact** — the u16 ceiling is gone
  end-to-end: compute wide, persist wide, read wide by name. (Value-cell u16 *returns* stay
  capped by the register convention — that's the honest residual, shown in the audit.)
  **Next here (updated):** u32 comparisons shipped (see above); retrieval rows for
  the wide siblings in the cell-eval dataset (library grew 98 → 100, so the
  `retrieval_compare` baselines will shift a hair on next run).
- **Fixed-point — a convention on `u32`, not a type. ✓ (core shipped).** Q-format is a `u32`
  (or `u16` for small ranges) with a point convention: a Q8.8 weight is `(a*w) >> 8` — a `u32`
  multiply + a shift the compiler already has. The convention is written down
  (`10-dialect-semantics.md`) and the helper cells **`q_mul`** / **`q_div`** ship (Q8.8,
  computed wide through `u32` so the 16.16 intermediate doesn't overflow; `q_div` is
  divide-by-zero-safe) — host-oracle rows (`tests/library.rs`), golden entries, and direct +
  paraphrase retrieval rows. A Q16.16 kernel needs a 64-bit intermediate the substrate lacks, so
  that stays the word-split `state`-cell pattern (documented). **✓ Optional structured manifest
  `scale` field — now shipped (`.cell` v7).** `//! scale: N` (a plain fractional-bit count, or a
  Q-format like `q8.8`) parses into `Manifest.scale: Option<u8>`, serializes as a presence byte +
  value (back-compat: pre-v7 reads as `None`), and surfaces in `inspect`'s human + JSON output so a
  host/agent reads a cell's values as `raw / 2^N` instead of guessing from the summary. `q_mul`/
  `q_div` declare `scale: 8`. A scalar cell stays `None` — the hash covers the field, so a scale
  change is a distinct artifact. Round-trip + parse tests; ABI doc updated.
- **Determinism split (write it down before the first wide op).** Integer / fixed-point /
  **softfloat** are all deterministic (softfloat is pure-integer IEEE-754, bit-identical
  cross-arch) → **in scope, gated on size/cost** (`max_code`, `trapped_ops`); the determinism
  clause is **never waived**. **Hardware float + transcendentals** are non-deterministic or heavy
  → **out**, the Wasm lane the non-goals reserve. `u64` on demonstrated eval need.
  **✓ The softfloat half shipped (2026-07-07, the F-waves — `docs/real-valued-cells-amendment.md`):**
  F0+F1 landed same-day — the kernel five + comparisons + conversions + rounding family +
  fmin/fmax as owned dialect-source kernels bit-identical to rustc f32 (H-F1 banks, both
  targets); the typed `f32` surface (suffix-required literals, operators/methods routing to
  kernels, no implicit int↔f32 anywhere) with the F0.6 canon guard landed as the sugar's
  precondition (H-F4 in CI); `finite_result` (`.cell` v8) turns non-finite f32 returns into
  typed escalations `0xFF07`/`0xFF08`; `Ty::F32` state fields (wire code 5); relocatable
  locals scratch (multi-kernel cells fit, byte-stable below the classic window; sandbox cap
  re-priced 4096→8192 = half the physical budget); measured cost table + banked negatives
  (barrel-jam slower than the loop; fadd 10,854 T) in `docs/10-dialect-semantics.md`; first
  hand-authored cells (`softfloat` pack: `norm2_f32`, `lerp_f32`) through admission with
  retrieval rows. **Owned transcendentals stay F2, demand-gated. Repr tags landed** (`plan::Repr` —
  the renderer type-flows representation like dimension; mixed-repr/q-mul/f32-exactness
  are named kills, f32 targets gate on finiteness, the battery perturbs f32 by +1.0):
  model-composed float *plans* are legal through the renderer; model-authored f32
  *source* stays hand-reviewed. **✓ F3 physics pack shipped (7 cells, finite-gated,
  bit-identical oracle, inverse-mass Rapier convention) and the resident kernel bank
  answered the pack's own demand signal one day after it was measured**: the
  arithmetic five + comparisons + helpers live once at `BANK_ORG = 0xC000` (bank
  locals at `0xB800`, disjoint from cell scratch), a banked cell's image carries only
  its own logic (`impulse_1d_f32` 8,197 B → 337 B, `elastic_collision_1d_f32`
  8,570 B → 650 B), and the bank's SHA-256 is pinned in the manifest (`.cell` v9;
  different bank ⇒ hard load error — never silently different arithmetic). Banking
  is opt-in (`//! kernel_bank: on`); inline cells keep their bytes and hashes.
  Still open: the bit-for-bit Rapier-trace validation (gates any video claim),
  `fma` (demand-gated), and the sandbox cap re-tighten once bank-by-default is
  decided.**
- **Cost honesty + DoD.** Wide / trapped ops are **counted in `trapped_ops` and gated** (capped,
  halted on budget), never folded into a cycles reward (extends the gate-not-gradient rule). The
  DoD grows one column: `Cell-target trap ≡ Spectrum-target software ≡ rustc`, under the
  fast-vs-authentic + cross-arch determinism fuzz. ✓ The prize was sized first (the
  `overflow_audit` example: 7/7 wrong → 2/7 after u32 arithmetic); `rustc ≡ Spectrum software`
  holds via `tests/diff/u32_ops.rs` and `Cell trap ≡ rustc` via the library host-oracle rows on
  the wide percent-family inputs.

### `rustz80` frontend — features the chuk-speccy authoring-plane kit needs

A second, *frontend*-side ask on the shared compiler, driven by chuk-speccy's authoring plane
(its spec 08, track E): **the subset must widen so composable SDK kit types and assets compile
*pure*** — one `impl Game` source → a host build **and** a bootable tape. The pure-Snake seam is
closed today (`chuk-speccy-sdk/samples/snake_game.rs` compiles both ways and boots on the real
ROM), but only inside a narrow envelope. Each item is a concrete blocker found while shaping the
SDK kit; the `file:line` is `rustz80` 0.5.0. These sit **below the agent-tool arc above** (the
eval is the gate, not VM/compiler features) but are tracked here since the compiler lives here.

- **Nested struct fields + field-of-field access — ✓ done.** `self.sprite.x` (and `a.b.c.d` to
  any depth). A struct-typed field lays out as its sub-struct's whole slot range
  (`FieldDef::struct_ty`, `lower/layout.rs`), field access recurses down the chain summing offsets
  to a scalar/`u32` leaf (`field_target` in `lower/expr.rs`, `FieldRef::field_struct`), and nested
  struct literals initialise the sub-fields (`store_struct_literal` in the new `lower/struct_init.rs`,
  recursive). Reading/assigning a *whole* struct field is a named error — access must reach a leaf.
  The composable kit shape works: `Sprite`/`Actor`/`TileMap`/`Hud` as fields, `&mut self` methods
  drilling in, `u32` leaves wide, and an array field *inside* a nested struct (`g.hud.cells[i]`,
  composes for free through the recursive `field_target`). Diff-tested against rustc on both targets
  (`tests/diff/nested_structs.rs`). **Nested struct field *inside* a struct-array element
  (`actors[i].pos.x`) — ✓ also done:** `elem_field_addr` generalised to `elem_field_chain_addr`
  (walks a member chain — element index + summed field offsets to a scalar leaf, at the leaf's
  width), `indexed_field_chain` collects the chain, and both struct-array init paths (the local
  `[Cell; N]` and the `[Cell; N]` *field*) route through the recursive `store_struct_literal`, so
  a `[Actor { pos: Point { .. }, .. }; N]` element lays out correctly. Byte-identical for existing
  single-level `a[i].field` (golden unchanged); oracle-tested for local + struct-field swarms and
  the whole-element / scalar-leaf rejections. Residual: a `u32` leaf of a struct-array element
  stays rejected (`LoadAt` has no wide form), as before.
- **Wider persisted struct fields — `u32` and signed `i16`. ✓ done.** A `u32` field lays out
  as two consecutive little-endian slots (`layout.rs`, `Width::DWord` — the ABI-v2 wide
  typed-state lane, drivable/readable by name at full width) and `i16` fields carry
  `Width::SWord`. A pure game's `u32` xorshift `Rng` can persist in state again.
- **Compact `[u8; N]` byte-array fields. ✓ done (2026-07-04, Phase S0).** A `[u8; N]`
  field **byte-packs**: `N` bytes in `ceil(N/2)` slots (`layout.rs`, `FieldDef::packed_len`),
  element access byte-addressed at `field_base + i` with real u8 semantics, `[v; N]` init
  as one slot `Fill`. `FieldLayout::bytes` reports it, so the cell layer never misreads a
  `[u8; 2]` field as a `u16` scalar (name-addressing as `bytes[N]` arrives with ABI v3).
  `[bool/i16; N]` fields stay one slot per element.
- **`&CONST → addr` — a const-data section. ✓ done (2026-07-04).** Top-level `const` items
  compile: scalars (`u16`/`u8`/`i16`/`bool`) substitute as literals; **data consts** —
  `[u8/u16/i16; N]`, `&str`, struct literals (`Tile { rows: […] }`), `[Struct; N]` — are
  **byte-packed into the image after the code** (`Ins::Bytes` at a `Def` symbol each, with
  its own DCE: only consts a kept function references are laid). `&TILE`, `&SHEET[i]`,
  `CONST[i]`, and string literals (interned, length-prefixed: at the time a u8 len
  byte; **since Phase S0 a little-endian u16** — `docs/11-machine-text.md` §1) all
  resolve by address (`Expr::ConstAddr` → `LD HL, sym`);
  `t: &[u8; N]` params read packed elements through the pointer, so a tile helper is *real
  Rust both ways* (diff-tested against rustc; 13 tests in `tests/diff/consts.rs`). New API:
  `lower_program_full` → `Lowered` + `codegen_loop_full` (old entries unchanged). The payoff
  landed in `chuk-speccy`: `Frame::tile(&HERO, …)` + `Frame::text(…, "SCORE")` route by
  address, verified drawing on the real 48K ROM — real sprite bitmaps **and a text HUD** in
  pure games, and a pure target for the `speccy-assets` tile pipeline.
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
`+=`, `for` ranges, local `[u8; N]`, structs/enums/methods. (The remaining *frontend* asks above —
nested struct fields (incl. `actors[i].pos.x` inside a struct-array element), signed/`u32` fields,
`[u8; N]` *fields*, `&CONST→addr` — are now all ✓ done. The composable SDK-kit type surface is
unblocked end to end.)

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
