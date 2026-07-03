# The escalation ladder — integrating the measured tool-calling stack

*(2026-07-03. Plan agreed across sessions; measurements live in chuk-soma
`docs/importance-cell.md`, `docs/roadmap.md` §B3′, and cell80's own eval history.
Division of labor at the bottom.)*

cell80's thesis is now **measured as economic, not capability-based**: a frontier
brain + the verifier folds everything the cell layer does — selection, composition,
even non-metric synthesis (99.2% on the B3′ surface) — at ~10⁶× the latency and real
token cost. So the product is one **cost-ordered escalation ladder**, monotone in
price all the way up, with the verifier testing every rung. Cheapest adequate
mechanism wins; the LLM is the last rung, not the first.

## The ladder

| rung | mechanism | cost | takes |
|---|---|---|---|
| 1 | fingerprint match (`rank_by_examples`) | µs | requests carrying I/O examples that an existing cell already satisfies |
| 2 | tiered retrieval (potion static → rerank) | µs–ms | text requests |
| 3 | synthesis search (`synthesize`, learned heuristic where gated in) | ms | outcome-specified requests over a known op family |
| 4 | **local brain + verifier loop** | seconds, zero frontier tokens | *shallow* leftovers only (see admission criteria) |
| 5 | frontier brain + verifier | $ + seconds–minutes | everything that survives 1–4; also authors metadata at register time |

## Item 1 — CellIndex tiering (potion → rerank → margin gate)

Measured baseline (cell-eval three signals): direct **1.00** / paraphrase **0.53** /
adversarial **0.50** for token-overlap; the firm-up run put static potion ~0.55–0.58
and MiniLM-class rerank ~0.72 **on the paraphrase split** — every tier must be
reported on all three splits, not a single P@1.

**The margin gate is the new work, and calibration is a first-class deliverable.**
A confidence margin is only as good as its placement, and the adversarial split is
the natural calibration set: choose the operating point so adversarial queries fall
into the *escalate* path rather than false-firing top-1 (the confident-cosine-0.98-
wrong-fact failure mode). Deliverable = the calibration curve (escalate-rate vs
split accuracy) + a chosen operating point checked into cell-eval, so re-runs catch
drift when the library grows.

## Item 2 — `cell_solve(examples)`: the ladder in one verb

Fingerprint first (an existing cell that already exhibits the behaviour beats
synthesizing it) → synthesis search → local-brain loop → frontier. A discovered
chain exports as a `CellGraph` and **registers as a new cell**.

**Register-back is where the library can quietly rot.** A synthesized chain is only
a usable tool if it's *findable*, and findability is authored metadata. At register
time the top-rung LLM authors name/description/tags **and discriminating probes**
(probes chosen where the new chain's outputs differ from its nearest fingerprint
neighbours — cheap on a deterministic substrate). Then the **admission gate**: the
new cell must pass the same paraphrase+adversarial retrieval gate as any library
cell before admission, or each demand-grown addition degrades P@1 for everything.

**Rung-4 admission criteria (priced by importance-cell §4b):** the local-brain
verifier loop measured 35% at depth-3 and 0% at depth-4+, burning ~50× one-shot
call cost for its solves. So rung 4 is *shallow-only and capped*: admit only tasks
that failed synthesis with small residual displacement, cap rounds hard, and route
anything deeper straight to rung 5. Uncapped, this rung is negative-value; capped,
it resolves a real fraction at zero frontier spend.

## Item 3 — index build runs TWO gates per op family

At `index build` time, per family:
1. **Heuristic earn-in** — train `ValueHeuristic` from the family's transition
   tables, run the learned-vs-hand gate (`synth_value_gate` shape), enable the
   learned heuristic only where it beats hand at equal budget. Persist the verdict.
2. **Admission gate** — every cell (authored or synthesized) passes the retrieval
   gate before joining the index.

Same pipeline point, same discipline: gates are CI, not one-off experiments.

## Item 4 — steps-specified composition: ergonomics only

Prompt names the steps → LLM authors the graph → verifier checks. Measured: it
folds; the bottleneck is that small models chain `cell_run` instead of authoring
graphs. Invest in graph-authoring affordances on the MCP surface, nothing
algorithmic.

## Item 5 — telemetry that measures the thesis

Per request: which rung resolved it + estimated tokens/latency of the LLM-only
counterfactual. Two additions:

- **Amortization, self-reported:** log value-net training cost per family alongside
  per-request savings, so the crossover claim ("the organ wins above N calls") is a
  number the product emits, not an assertion.
- **Clock integrity, pulled forward:** the "microseconds and zero tokens" pitch
  makes the T-state / host-trap wall-cost conflation product-facing. Land the
  cycle-accounting fix WITH the telemetry — a latency column on the wrong clock is
  worse than none.

## Do-not-build (banked negatives)

No learned selector (three measured losses to retrieval). No search on pipelines
(they fold). No MCTS (deterministic substrate — best-first/A* is the one organ).
No capability marketing for synthesis (the frontier folds it; the pitch is price).

## Ordering + division of labor

- **CellIndex tiering: proceeds now, in the u32/compiler session** — orthogonal to
  the `Ins`/Stage-2 churn, no coordination cost.
- **`cell_solve` + register-back: held until the u32/Ins branch merges** — CellGraph
  export and the library surface are exactly where that branch is churning.
- Gate-as-build-step lands with whichever of the above first touches index build;
  telemetry (+ clock fix) last.
