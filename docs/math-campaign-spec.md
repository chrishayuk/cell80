# The math campaign — GSM-Symbolic as the first `cell_solve` field campaign

*Status: M1 pack 3/5 landed (checked/exact arithmetic, money/basis-points, units —
`docs/library-growth.md` "GSM8K math campaign"); M0 confirmed still open
(u32-across-a-call-boundary, tested directly and still rejected even after `Cond32`
landed, blocking the fraction pack specifically); M2-M4 (the plan IR, renderer, and campaign
itself) remain gated behind `cell_solve` (`docs/escalation-ladder.md` item 2 — the u32/Ins
compiler branch it was held on has since landed, so the compiler prerequisite for
`cell_solve` is clear, but `cell_solve` itself is not built) and the admission gate (Phase
2.2, shipped). This replaces an earlier "GSM8K Ace Pack" draft: same evidence base,
different shape. That draft proposed cell80 acquire a math runtime to chase a benchmark.
This spec proposes the inverse: **the benchmark is the first field campaign for the loop
cell80 already built.** Nothing here is a new subsystem; everything is the existing
thesis — compile, admit, retrieve, memoize, escalate — pointed at a domain with 8,500
labelled tasks and two purpose-built robustness suites.*

## Delta from the original "GSM8K Ace Pack" draft

**Killed:** the `plan_run_*` interpreter (a VM inside the VM, no content address, no
admission — plans compile instead); the 80-cell hand-authored schema pack (schemas
precipitate from the campaign via artifact-hash dedup and register-back — H-M3 makes that
measurable rather than hoped); the i32 cell lists (not in the dialect — sign-magnitude
convention cells plus escalate, with the corpus's escalation rate deciding whether i32 ever
earns dialect entry); the MATH/AIME packs (gated behind this campaign reading out); and the
word "ace" (frontier models are at ~95% on GSM8K; the unsaturated question is small models
under perturbation).

**Kept, because the original had them right:** exact-checked-arithmetic first, basis points
over float percentages, the unit-dimension tags, the verifier/ranker orientation, the plan
IR itself — demoted from executable to wire format between model and renderer — and the
counterfactual battery, arguably the single best idea in the original, since
re-execution-too-cheap-to-ration is the one capability that's *only* natural on this
substrate.

**Added:** PAL/Program-of-Thought named as prior art up front, so the claim is scoped to
what cells add over Python (units, exact rationals, µs sweeps, memoization, residue) rather
than re-proving a 2022 result; a pre-registered four-hypothesis grid with kill criteria,
including H-M2 stating *now* that accuracy parity with PAL-Python is the expected outcome
(so robustness-per-dollar is the claim, not a retreat); and a renderer-determinism
requirement (canonical quantity ordering) — the small technical detail the whole
precipitation story hangs on, since identical schemas must hash identically or H-M3 is
unfalsifiable.

Authored surface drops from ~175 cells to ~95; the prerequisite list gets shorter but
harder — u32-across-a-call-boundary is now demand-confirmed by the fraction pack rather
than speculative, so it graduates to an M0 blocker.

## Why this domain, stated honestly

GSM8K is saturated for frontier models (~95%+ with plain CoT); "acing" it with a big model
in the loop proves nothing anyone doubts. What is *not* saturated:

1. **Small models under perturbation.** GSM-Symbolic and GSM-Plus show accuracy degrading
   when numbers change, clauses are inserted, or targets move — worst for small models. A
   3B that holds flat where a 26B degrades is a result.
2. **Cost per verified answer.** Nobody measures it. We can, exactly, in T-states and
   tokens.
3. **What a benchmark run leaves behind.** Every prior system's artifact is a score. Ours is
   a library plus a fact file.

So the headline configuration is **granite 3B + compiled plans**, versus the same model on
CoT and on PAL-style Python — the LARQL thesis (frontier-class capability on consumer
hardware) in benchmark clothes. PAL / Program-of-Thought established in 2022 that
LLM-extract → deterministic-execute beats CoT, with Python as the executor; the claim here
is never "execution beats reasoning," it's what cell80 adds **over Python as the
executor**: typed unit flow, exact rationals by default, µs re-execution making candidate ×
perturbation sweeps free, content-addressed identity making the whole campaign memoizable,
and the register-back residue. Robustness and cost, not raw accuracy — if accuracy parity
with PAL-Python is all we get, the claim survives, stated now rather than retreated to
later.

## The architecture: compile the plan — no interpreter

```
problem text
  → LLM extracts: quantities (typed, unit-tagged), candidate plan(s), target   [plan IR]
  → host renders each plan as trivial dialect Rust (quantities = state fields)
  → rustz80 compiles it (sub-ms)                                    [the plan IS a cell]
  → runner executes + verifies + perturbs, memoized                 [µs each, cached]
  → survivors answer; recurring structures register back through the gate
```

Consequences, each doing real work:

- **Perturbation is a field sweep.** Quantities are state fields, so GSM-Symbolic's entire
  counterfactual battery is *same cell, different inputs* — one compile, hundreds of runs
  at batch speed, every one a cache candidate.
- **Schemas precipitate; nobody authors them.** Two problems with the same structure and
  different numbers render to the same Rust modulo constants — same artifact hash. The
  original's ~80 hand-written schema cells are gone from the plan: the campaign *discovers*
  which schemas recur, and register-back admits them (ACT-R production compilation,
  measured: solved deliberatively once, retrieved reflexively forever — the precipitation
  count is a headline metric, not a hope).
- **The interpreter-vs-schema tension dissolves.** The original bet on both an opcode IR
  and a retrieval schema pack, which compete. Here there is one path: compile. Retrieval
  enters only when a new problem's structure matches an admitted cell — behavioural
  routing, by feeding the quantities and checking the output, not by prose.
- **Verification rides shipped machinery.** A surviving plan's runs are facts; the
  campaign's by-product is a `.facts` file of verified grade-school mathematics.

The plan IR survives as the right extraction target (MathQA's operation-annotation design
is the precedent), but it is a *wire format between model and host renderer*, never
executable:

```json
{ "quantities": [ {"id":"lego_sets","value":13,"unit":"count"},
                  {"id":"lego_price","value":1500,"unit":"cents_per_count"} ],
  "ops":        [ ["mul","lego_sets","lego_price","lego_money"] ],
  "target":     "sets_sold",
  "constraints":[ ["nonneg","sets_sold"], ["exact_div","needed_money","lego_price"] ] }
```

The renderer is deliberately dumb: one op → one line of Rust; units checked symbolically at
render time (a `unit` is a tag or tiny exponent vector — dollars plus hours fails *before*
compilation); constraints render as trailing checks that `Escalate` on violation. Renderer
output is deterministic and canonical (sorted quantity order) — what makes
hash-precipitation work.

## Prerequisites — shorter but harder than "175 cells"

1. **One u32 across a call boundary** (demand-confirmed by the fraction pack: `frac_add`
   needs a `gcd_u32` kernel it currently cannot call across a function boundary within one
   compiled unit). Lands first; without it the fraction pack is copy-paste.
2. **Signed-32 decision: convention before dialect.** `i32` is not in the dialect and this
   campaign does not add it. GSM-scale quantities are non-negative except transiently in
   differences; the renderer emits sign-tracked u32 (sign-magnitude convention cells:
   `smag_sub`, `smag_cmp`) and `Escalate::NeedsWiderMath` where that's insufficient. If the
   corpus's escalation rate says otherwise, *that measurement* — not this spec — opens the
   i32 dialect question.
3. **The wide-literal fix** (`let w: u32 = 100000;`) — the renderer will hit it in its
   first hundred plans.

No other dialect surface: no floats (fractions and basis points are the point), no strings
(the model reads; cells compute), no plan opcodes.

## The authored packs — ~95 cells, not 175

What still deserves hand-authoring is what plans *call*, not what plans *are*:

- **checked/exact arithmetic (~30)** — checked add/sub/mul at u32, exact and floor/ceil
  division with remainder surfaced (a nonzero remainder where the plan declared
  `exact_div` is a *wrong plan*), `fits_u16/u32` guards, sign-magnitude kernels.
- **fractions (~20)** — u32 numerator/denominator, eager reduction after every op
  (grade-school denominators stay tiny), `Escalate::NeedsWiderMath` on overflow rather than
  approximation. `frac_add/sub/mul/div/cmp/eq`, `is_integer`, `floor/ceil`, `to_mixed`,
  ratio splits.
- **money & basis points (~15)** — cents arithmetic, `discount_bps`/`tax_bps`/`markup_bps`,
  `original_before_discount` and inverse-percent family. Basis points, never float
  percentages.
- **units (~10 + render-time checker)** — dimension tags (`count, money, time, distance,
  area, volume, rate`), `unit_mul/div/cancel_check`, `same_unit_check`. Most unit bugs die
  at render time; the cells exist so *plans* can carry explicit unit assertions that
  survive into the compiled artifact.
- **verifier/ranker (~20)** — `answer_eq`, `answer_in_options`, reverse-equation
  satisfaction, nonneg/integer/range constraint checks, multi-plan agreement, tie-breaks —
  the pack the original GSM8K verifier paper is precedent for: generate k plans, keep the
  ones that survive execution.

Everything else from the original draft — the 80-cell schema pack, combinatorics, geometry
formulas, number-theory extensions, polynomial mini-pack — is **deferred to demand**:
schema cells precipitate (above); contest-math packs wait for a MATH/AIME campaign
explicitly out of scope here and gated on this one reading out.

## The loop, operationally

Per problem: extract quantities + k plans (k = 5–10) → render/compile each (rejects are
repair-eval data) → execute all, memoized → **kill plans** that fail unit flow, leave a
remainder under `exact_div`, go negative on a declared-nonneg, contradict a stated
total/leftover, or fail the reverse equation → if one survives, answer; if several, run the
**counterfactual battery** (perturb quantities: does each plan's answer move consistently?
insert an unused-quantity check: does the plan ignore it? flip the target: does the target
slot move?) and keep the invariant one; if none survive, escalate up the ladder — a
frontier-authored plan re-enters through the same renderer and, if novel and recurrent,
through the gate.

The counterfactual battery is the one capability here that's *only* natural on this
substrate — re-execution too cheap to ration — aimed squarely at what GSM-Symbolic
measures.

## The eval — pre-registered

Configurations: granite-3B {CoT, PAL-Python, compiled-plans}; gemma-26B {same three};
frontier reference (CoT). Corpora: GSM8K test, GSM-Symbolic, GSM-Plus, SVAMP.

Metrics: accuracy · **perturbation degradation** (accuracy delta, original → perturbed,
the headline) · cost per solved problem (tokens + T-states + wall-clock) · plans killed by
each verifier class (which checks earn their keep) · **schemas precipitated** (distinct
admitted artifact hashes / problems seen — the procedural-memory curve) · facts banked ·
escalation rate by rung.

Pre-registered hypotheses, with kill criteria:

- **H-M1 (robustness):** 3B+cells degrades ≤ half as much as 3B-CoT on GSM-Symbolic.
  *Kill:* degradation statistically indistinguishable → the fragility is extraction, not
  arithmetic; banked negative, campaign narrows to the verifier role.
- **H-M2 (parity):** 3B+cells accuracy ≥ 3B+PAL-Python. *Expected marginal* — plan quality
  is extraction quality — stated so; the differentiators are H-M1, cost, and residue.
- **H-M3 (precipitation):** the schema curve bends — precipitated cells are retrieved in
  preference to fresh compilation at a growing rate across the corpus. *Kill:* every
  problem compiles fresh and nothing recurs → the procedural-memory claim fails in its best
  domain; that finding outranks any accuracy number.
- **H-M4 (cost):** cost per verified answer for 3B+cells beats every configuration that
  matches its accuracy.

Honest limits: cells fix arithmetic, not reading — extraction remains the bottleneck;
GSM-Symbolic fragility is partly a *reading* failure no executor repairs; MATH/AIME remain
helper-territory, out of scope, gated. The word "ace" does not appear in this spec's claims.

## Sequencing gates

| gate | contents |
|---|---|
| **M0** | u32-across-calls · wide-literal fix · sign-magnitude decision recorded |
| **M1** | arithmetic/fraction/money/unit/verifier packs through the admission gate (with retrieval rows and probes, per the contribution rule — these are library cells like any other) |
| **M2** | plan IR + renderer + counterfactual battery; renderer determinism test (same plan → same hash); repair rows from renderer rejects |
| **M3** | the campaign: full grid, metrics published, precipitated schemas admitted, `.facts` exported |
| **M4** | read-out decides: extend (contest packs) / narrow (verifier-only) / bank |

## The one-sentence version

Don't build a math runtime to pass a benchmark — run the benchmark through the runtime you
built: plans compile to cells, schemas precipitate into memory, every answer leaves a
verifiable fact behind, and the headline is a small model that stops flinching when the
numbers change.
