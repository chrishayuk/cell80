# cell80 — Roadmap

> **Execution plan:** [roadmap-phases.md](roadmap-phases.md) sequences the work as
> phased gates (0: determinism contract ✓ → 1: LLM-facing compiler → 2: retrieval →
> 3: trust → 4: codegen stage 2), with a DoD per item and the end-state narrative.
> This file stays the ledger of what's *built*.

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
   **Found in practice, not yet fixed: `Fingerprint::compute`'s stateless branch only digests
   the primary (`HL`) register** (`cell80/src/fingerprint.rs`: `Some(r.result)` when
   `state.is_empty()`), never `DE`/`BC` — unlike the state-cell branch two lines below it,
   which already digests every named output field via `digest_state`. A stateless free
   function that returns a `(u16, u16, u16)` tuple is therefore fingerprinted as if it only
   returned its first element: the admission gate refused a scoped `sort3` cell (`(min, mid,
   max)` in one call) as a duplicate of `min3` at 1.00 agreement, correctly by the fingerprint's
   own math (`min3`'s single output *is* `sort3`'s first tuple slot, for every input) but
   missing the point that `sort3`'s real payload — `mid` and `max` — lives entirely in the two
   registers this branch never looks at (`docs/library-growth.md`'s "sort3" pack note has the
   full account). The fix is narrow and stays inside this same function: when the entry's
   declared return type is a tuple, digest `r.regs` (or however many slots the signature
   declares) instead of just `r.result`, the same "digest the whole output surface" principle
   the state-cell branch already applies. Nobody has needed a tuple-returning free function
   enough to justify it yet — `sort3` is the first cell this has ever blocked, and it wasn't
   shipped, so the current 239-cell library has zero live cells riding on the fix. Whoever next
   wants to ship a tuple-returning free function alongside an existing same-first-value cell
   hits this for real; until then it's a known, scoped, low-risk fix waiting for a reason.
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
6. **Grow the standard cell library — 209 cells across 30+ families (2026-07-05).**
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
   tradeoff checkpoint by checkpoint.
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
   `docs/math-campaign-spec.md`. **Next** — M3, the actual field campaign (a real corpus, a
   small model in the loop, cost measured in T-states/tokens against CoT and PAL-Python
   baselines) — not started; this item is infrastructure + a smoke test, not the campaign
   itself.

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
  fired-proof shape assertion (`tests/diff/bytes.rs`). **Parked (the bigger win):** a full
  A-accumulator evaluator for chained byte arithmetic (avoids the per-op `PUSH HL`/`POP DE` spill) —
  a larger, riskier rewrite of the HL-accumulator core, deferred until it can be measured end-to-end
  in a byte-heavy chuk-speccy build. Byte add/sub already compute the low byte optimally in `HL`.
- **Next rules worth ranking:** window-spanning leaf pairs (the `ptr_elem_addr` shape — the
  pop is separated from the push by an effect-free span), `INC HL`/`DEC HL` strength-reduction
  for `± 1`/`± 2` literals (flags differ from `ADD` — needs the no-flag-consumer argument made
  explicit), and a *what's-in-`HL`/`DE`* tracker for cross-statement reload elision.

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
  (`tests/diff/nested_structs.rs`, 8 cases + rejections). **Follow-on:** a nested struct field
  *inside* a struct-array element (`actors[i].pos.x`) — the `[Cell; N]` initialiser and
  `elem_field_addr` step only one field level; guarded with a named error, not a silent one-word read.
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
nested struct fields, signed/`u32` fields, `[u8; N]` *fields*, `&CONST→addr` — are now all ✓ done;
the one open follow-on is a nested struct field *inside* a struct-array element, `actors[i].pos.x`.)

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
