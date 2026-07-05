# The math campaign — GSM-Symbolic as the first `cell_solve` field campaign

*Status: **M1 complete, second slice landed** — all five authored packs landed a first
slice (checked/exact arithmetic, money/basis-points, units, verifier/ranker, fractions —
`docs/library-growth.md` "GSM8K math campaign"), then a second slice (checked-arithmetic
+18, fractions +9, money-bps +2, verifier-ranker +11, plus a units dimension-table fix)
closed most of the gap against this spec's original ~95-cell estimate (32 → 73 authored
cells across the five packs) — see `docs/library-growth.md`'s "second slice" note for what
was built, what was deliberately not (money-bps/units candidates already covered), and the
retrieval-curve cost it paid. M0 (u32-across-a-call-boundary) landed from a parallel
session as Tier 2 (one u32 param per call, not the two a shared two-u32-param `gcd_u32(a,
b)` *free-fn reducer* needs); the fractions pack shipped anyway by inlining its own
GCD-reduction loop per cell instead, a workaround available even before M0 landed. The
second slice's own `gcd_u32` **cell** (a wide sibling of `gcd`) is this same
inline-loop workaround packaged as a standalone library entry, not the blocked
two-u32-param shared reducer — the two aren't in tension. M2-M4 (the plan IR, renderer, and
campaign
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

1. **One u32 across a call boundary** — landed (Tier 2: one u32 param per call, must be
   first; a u32 return; confirmed *not* enough for a shared two-u32-param `gcd_u32`
   kernel, which still can't be called). The fraction pack shipped anyway: each cell
   inlines its own GCD-reduction loop, so it *is* copy-paste, as predicted — just via a
   workaround available even before this landed (a `while` loop over u32 locals inside
   one function was never gated by the call-boundary limit), not because the limit was
   fully lifted.
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
| **M2** | plan IR + renderer + counterfactual battery; renderer determinism test (same plan → same hash); repair rows from renderer rejects — **shipped 2026-07-05** (`cell80/src/plan.rs` + `CellHost::solve`; CLI `cell80 solve`, py `solve`, MCP `cell_solve`; determinism/kill/battery tests in `cell80/tests/plan.rs`; the wide-literal prerequisite fixed the same day; render rejects surface as `kill: render/compile:` strings — repair rows harvest from a pilot run). Plan cells stay **warm** — one runner per schema, so a re-seen schema is *retrieved* (the `retrieved` flag is H-M3's counter) and its runs land in the fact file. |
| **M3** | the campaign: full grid, metrics published, precipitated schemas admitted, `.facts` exported |
| **M4** | read-out decides: extend (contest packs) / narrow (verifier-only) / bank |

## M2 test-drive, hand-crafted (2026-07-05) — two findings before M3 runs for real

Before spending a model + a corpus on M3, `cell_solve` was exercised directly against a
handful of hand-crafted word problems (`cell80/examples/solve_wordproblems.rs`, kept as a
runnable check) — not the campaign, just "does the loop do what §The architecture claims."
Every plan answered correctly, exact_div violations killed cleanly, and genuine
multi-plan disagreement resolved via the battery, matching the shipped test suite. Two
things worth knowing *before* M3 spends real compute on them:

1. **Precipitation is literal-field-name-sensitive.** Two independently-authored word
   problems sharing a structure ("N items at a unit price" — a notebook problem and a
   pencil problem, different numbers) hashed identically and the second was correctly
   `retrieved`, *when both used the same generic quantity ids* (`qty`, `unit_price`,
   `total`). The identical pencil problem re-extracted with its own natural nouns
   (`pencils`, `pencil_price`, `pencil_total`) rendered to different source and never
   matched the generic version — because quantity ids become literal struct field names,
   and the renderer only canonicalizes their *order*, not their *spelling*. H-M3's
   precipitation count will undercount real schema recurrence unless the model-facing
   extraction step normalizes quantities to canonical role names before rendering, not
   whatever nouns the problem text used. Not a bug in `plan.rs` — a real requirement on
   the extraction prompt/step that M3 hasn't had to satisfy yet, since nothing has run
   more than one hand-picked plan through it before now.

2. **The counterfactual battery only fires on disagreement, not to verify agreement.**
   Constructed case: two candidate plans, `mul(a,b)` and `add(a,b)`, at `a=b=2` — both
   equal 4, a coincidence at these specific numbers (the same class of failure this
   project has hit before, the documented `min`/`median3` register-0 coincidence).
   Because `solve`'s consensus check (`cell80/src/plan.rs`, the `answers.windows(2).all(…)`
   arm) short-circuits to "they agree, done" *before* ever perturbing, the battery never
   ran and the coincidence would have been accepted as consensus silently. This is the
   opposite of the failure mode the shipped test `counterfactual_battery_separates_
   coincidental_agreement` covers (genuine disagreement, correctly resolved) — that test
   doesn't exercise agreement-that-should-have-been-checked at all.

   **Fixed:** deleted the early-agreement arm so the full perturbation/grouping logic
   always runs whenever more than one plan survives, agreement or not:
   ```diff
        let answer = match live.len() {
            0 => None,
            1 => answers[0],
   -        _ if answers.windows(2).all(|w| w[0] == w[1]) => answers[0],
            _ => {
   ```
   All 8 pre-existing `cell80/tests/plan.rs` cases pass unchanged (none relied on the
   shortcut — the one multi-plan agreement test already disagrees pre-perturbation and
   takes the other arm either way), and a new regression test,
   `counterfactual_battery_also_fires_on_a_coincidental_pre_perturbation_agreement`, locks
   in the `mul`/`add` coincidence resolving to `None` (escalate) instead of a silent
   `Some(4)`. The tradeoff is real and accepted: every multi-plan-agreement case now
   always perturbs (a small, bounded cost — one extra `run_state_fast` per quantity per
   surviving plan), trading a bit of throughput for the battery actually doing what
   §"The architecture" claims it does.

## A real smoke test (2026-07-05) — 73 genuine GSM8K problems, still not M3

`cell80/examples/m3_gsm8k_smoketest.rs`, kept as a runnable check: the first 77 rows of
`openai/grade-school-math`'s public `test.jsonl` (fetched via the raw file directly after
a lossy summarizing fetch started truncating past ~20-40 rows — not written to match a
known schema, not cherry-picked, not filtered for ease — see below for the 4 that got
skipped and why), hand-extracted into the plan IR by reading each English problem and doing
the extraction the spec asks a model to do. **Read the caveats before the result** — this is
emphatically not M3: one extractor (me, not a 3B model), N=73 not 1,319, no distractor/
wrong-plan candidates, no cost measurement, no CoT/PAL-Python baseline. What it *is*: a
meaningfully larger, still fully-verified check of this project's rendered-plan loop against
a real, unfiltered, consecutive slice of the benchmark.

**Result: 73/73 correct** (started from 77 consecutive rows; 4 skipped as genuinely
unrepresentable in the current plan IR, not silently dropped — see the findings below —
a ~95% representability rate on this slice), spanning 2-8 op chains: subtraction, exact
percentage math (the `scalar`-unit `mul`-then-`div` pattern, `value * pct / 100`, exact at
every problem's numbers, including percent-of-a-percent and reverse-percentage cases),
rate/time flows (`count_per_time` and `distance_per_time` correctly inverting to `time` on
division), several reverse-chain algebra problems (Melanie's vacuum cleaners, Gretchen's
coins, Candice's post-its — "work backwards from the ending state to the starting
quantity"), and one genuinely fiddly unit-rescale (Uriah's book bag: quarter-pound units
resolve `0.25lb`/`0.5lb` exactly, the same fixed-base-scale lesson as the cents convention,
generalized to weight). Two hand-perturbed variants (same ids, new numbers — James's
sprints, the sheep problem) both correctly precipitated (`retrieved: true`) and answered
correctly. None of the 73 distinct problems precipitated against each other, as expected —
they're 73 genuinely different problems, not variations of one; H-M3's precipitation curve
needs a real corpus with real recurring structure to show anything, which independently-
different problems can't provide by construction, however many of them there are.

Five things worth carrying into a real M3 design:

- **Genuine ambiguity exists even for a careful extractor.** Josh's house-flip problem
  ("increased the value of the house by 150%") admits two readings — *the increase equals
  150% of the original* (the ground truth's own reading: `+120,000` on top of `80,000`) vs
  *the new value is 150% of the original* (`120,000` total, a very different answer). I
  matched the ground truth's worked solution, but a smaller model resolving this from the
  English alone, with no worked solution to check against, is exactly the kind of case
  H-M1 (perturbation robustness) should expect to find fragile — extraction ambiguity, not
  arithmetic, same honest limit the spec's own "Honest limits" paragraph already names.
- **The dialect's 4 base dimensions (`count`, `money`, `time`, `distance`) need a fallback
  convention for everything else** — GB, cups, sprints, glasses, sheep all got mapped to
  plain `count` here, and rate-shaped quantities (`meters_per_sprint`, `cups_per_chicken`)
  needed the less-obvious `X_per_count` / `count_per_count` unit spelling to keep add/sub's
  dimension check happy downstream. That mapping instinct ("no dedicated dimension → treat
  as `count`, unless the problem divides by it") is something a real extraction prompt
  needs stated explicitly, not left implicit — I had it because I could reason about the
  renderer's dimension-checker directly; a model extracting blind from problem text alone
  won't reconstruct that convention on its own.
- **The plan IR has no comparison or decision primitive at all.** Row 16 (a merchant
  choosing between two investments, "pick whichever profit is bigger") can't be rendered:
  [`PlanOp`](cell80/src/plan.rs)'s `op` field only accepts `add`/`sub`/`mul`/`div` — there is
  no `max`/`cmp`/branch opcode, and no way to add one without extending the renderer and its
  render-time unit checker together. This is a real, not hypothetical, gap: comparison-
  shaped word problems are a normal GSM8K category, not an edge case. Two paths forward,
  neither built here: extend the plan IR with a comparison op (more renderer surface, more
  determinism risk to verify), or detect comparison-shaped problems at the extraction step
  and route them to library cells directly (`is_gt`, `max`, `choose_best3`) instead of
  through a rendered plan — the second matches the project's existing division of labor
  (hand-authored cells for reusable primitives, rendered plans for the arithmetic glue
  around them) better than growing the IR.
- **Fractional dollar amounts need a firm cents-always convention, not per-problem
  judgment — found, then fixed in the smoke test itself.** The first pass used
  `unit: "money"` for whole-dollar problems (Josh's house, Kylar's glasses) and only
  rescaled to *cents* where the English forced it (three `$16.50`-style problems — Kyle's
  book, Marie's pizza, Mishka's clothes): same unit string, two scales, each internally
  consistent but not with each other. `render()`'s dimension checker never caught it (it
  only checks *within* one plan, never compares scale *across* plans), so every problem
  still compiled and answered correctly — the bug was silent, not a compile error. Every
  money-valued problem in `cell80/examples/m3_gsm8k_smoketest.rs` is now in cents
  throughout (still 25/25 correct after the rescale), demonstrating the fix rather than just
  naming it: one firm rule stated once, not per-problem judgment re-derived each time — the
  same lesson the shipped money-bps pack already encodes for the hand-authored cells
  (`docs/library-growth.md`'s "cents" naming note), now also true of the plan-extraction
  side. The lesson generalizes past money: any unit with a real-world sub-integer step
  (fractional time is the same shape — row 9's skipped half-hour problem) needs its base
  scale fixed *before* extraction starts, not discovered mid-corpus. Row 53 (Uriah's book
  bag, quarter-pound units) shows the lesson isn't "avoid fractions" — it's "check every
  fraction in the problem shares a rescale-able common denominator first."
- **A real renderer bug, found and fixed: the identifier blocklist didn't cover Rust's
  reserved-for-future-use keywords.** Row 70 (Bailey's allowance) naturally wanted a
  quantity named `final` ("ends with a final total of $100"). `render()`'s own `ident_ok`
  check (`cell80/src/plan.rs`) is supposed to catch bad identifiers with a clean,
  named error *before* compilation — but its blocklist only covered `self`/`run` and a
  handful of keywords actually hit in practice (`fn`/`let`/`mut`/`if`/`else`/`while`). `final`
  isn't a keyword Rust's grammar assigns a meaning to today, but it's reserved for future
  use, and `syn` accepts it as a keyword token regardless — so it fell through the
  incomplete blocklist and hit a raw `rustc` parse error instead of the clean render-time
  message the checker exists to give. **Fixed:** the blocklist now covers Rust's full
  strict + reserved keyword set reachable in the lowercase-identifier charset (`as`
  through `while`, plus `abstract`/`become`/`box`/`do`/`final`/`macro`/`override`/`priv`/
  `typeof`/`unsized`/`virtual`/`yield`/`try`/`union`), with a regression test in
  `cell80/tests/plan.rs` locking in the clean rejection. Unlike the counterfactual-battery
  fix, this one had no design tradeoff to weigh — a stricter, more complete identifier
  check is strictly safer, so it was applied directly rather than left proposed.

## MATH/AIME — scoped ahead of the gate (2026-07-05)

Chris asked to look at the MATH and AIME benchmarks and sketch what math functions a future
pack would need. The gate above still holds — nothing here is authored or admitted; this is
the research the spec's own "deferred to demand" line asks for, written down so it doesn't
have to be redone when M4 actually opens this door.

**What the two benchmarks actually are.** MATH (Hendrycks et al.) is 12,500 competition
problems (12,000 train / 500 test) across seven subjects — Prealgebra, Algebra, Number
Theory, Counting & Probability, Geometry, Intermediate Algebra, Precalculus — difficulty 1-5,
answers frequently fractions, radicals, or symbolic expressions, not bare integers. AIME
(American Invitational Mathematics Examination) is 15 problems per sitting, two sittings a
year, and by contest design **every answer is an integer 0-999** — algebra, geometry,
combinatorics, and number theory, with "find the remainder when N is divided by 1000" as a
recurring finishing move.

That last fact is the load-bearing one for this dialect: AIME's integer-answer contract
matches u32-and-no-floats natively; MATH's doesn't. A large fraction of MATH's Precalculus,
Intermediate Algebra, and much of its Geometry (trig, circles, complex numbers, irrational
radicals as final answers) fails the same "no floats" rule the GSM8K spec already drew — not
a new limitation, just one MATH hits far harder than GSM8K ever did. **If this door opens, an
AIME-shaped pack is the tractable slice; full seven-category MATH mostly isn't**, without a
much bigger dialect change (rationals-with-irrational-tags, at minimum) than anything this
project has scoped.

**Category-by-category, against the existing 209-cell library** (`docs/cell-index.md`):

- **Number theory — the biggest real gap.** `gcd`/`gcd3`/`lcm`/`lcm3`/`is_coprime`/
  `is_prime`/`isqrt`/`digit_sum`/`num_digits`/`pow_mod` already exist, but `pow_mod`'s
  `m <= 256` ceiling (u16 domain, the intermediate squared value must fit u16, i.e.
  `m*m <= 65535`) is too small for AIME's mod-1000 finishing move — a `pow_mod_u32` (u32
  width, intermediate squared value as u32, supports `m` up to ~65535) is the single
  highest-value candidate here. Alongside it: `mod_add_u32`/`mod_sub_u32`/`mod_mul_u32`,
  `is_prime_u32` (wide sibling, same reasoning as `gcd_u32`), `count_divisors`,
  `sum_divisors`, `euler_totient`, `smallest_prime_factor`, `digit_reverse`,
  `digit_product`. `mod_inverse` (extended Euclid) and `crt_solve_pair` (two-congruence
  Chinese Remainder) are real AIME techniques but noticeably more complex to author
  correctly — stretch items, not a first slice.
- **Counting & Probability — the second real gap.** Nothing like `factorial`, `choose`
  (nCr), or `permute` (nPr) exists yet. `factorial_checked_u32` escalates past `n=12`
  (`13!` overflows u32); `choose_u32`/`permute_u32` need the multiplicative
  running-division formula (`result = result * (n-k+i) / i`, exact at every step) to avoid
  overflowing before reducing, the same overflow-avoidance shape `frac_reduce`'s inline
  GCD already models. Probability itself likely doesn't need new cells — it's
  `choose`/`factorial` composed through the existing `frac_*` pack.
- **Prealgebra / Algebra — mostly already covered.** These overlap heavily with the GSM8K
  packs (checked arithmetic, fractions, money/bps). The one AIME-flavored addition is
  Vieta's formulas (a quadratic's root sum/product as `-b/a`/`c/a`) — but that's `frac_div`
  composed with existing checked ops, a plan-level composition, not a new cell.
- **Geometry — partially tractable, integer-coordinate only.** `dist_sq` (avoids `isqrt`
  for problems that only ever compare or sum squared distances) and `shoelace_area_x2`
  (twice a polygon's area from integer vertices — always an integer, unlike the raw area)
  are plausible; Pythagorean-triple checks (`a*a+b*b==c*c`) are composable from existing
  `mul`/`add`/`eq`, the same call `is_perfect_square` already gets (compose, don't
  author). Anything needing π, trig, or a non-right triangle's exact area is out.
- **Intermediate Algebra / Precalculus — mostly out of dialect.** Symbolic polynomial
  roots, trigonometric identities, complex numbers, and irrational final answers all fail
  the no-floats rule outright. Same deferral the original spec already gave the
  "polynomial mini-pack."

**Net candidate list, if/when M4 opens this door** — number theory: `pow_mod_u32`,
`mod_add_u32`/`mod_sub_u32`/`mod_mul_u32`, `is_prime_u32`, `count_divisors`,
`sum_divisors`, `euler_totient`, `smallest_prime_factor`, `digit_reverse`, `digit_product`;
combinatorics: `factorial_checked_u32`, `choose_u32`, `permute_u32`; geometry: `dist_sq`,
`shoelace_area_x2` — roughly 15 cells, all AIME-shaped, none requiring a dialect change,
smaller and more tractable than the original ~95-cell GSM8K estimate. It should start from
an AIME-only corpus, not full MATH's seven categories.

**This changed nothing about the gate — until Chris explicitly said to proceed anyway.**
`docs/library-growth.md`'s "math-cell growth pauses here on purpose" and this section's own
gating both still describe the *default* plan: M3 (a real corpus through `cell_solve`)
hasn't run, so precipitation hasn't had its say, and the list above was guessed, not
demonstrated. Asked directly to build the list rather than just scope it, 12 of the ~16
number-theory/combinatorics/geometry candidates named above landed as real cells
(`docs/library-growth.md`'s "MATH/AIME pack, first slice" note has the full account,
including two things that broke on first pass — `choose_u32`'s formula transiently
overflowing before the true answer would, and a struct field rejecting direct `if`/`else`
assignment): `pow_mod_u32`, `mod_add_u32`/`mod_sub_u32`/`mod_mul_u32`, `sum_divisors`,
`euler_totient`, `smallest_prime_factor`, `digit_reverse`, `digit_product`,
`factorial_checked_u32`, `choose_u32`, `permute_u32`. Four of the originally-scoped names
didn't ship: `count_divisors` and `dist_sq` turned out to be exact duplicates of
already-landed `factor_count`/`euclid_sq` (caught by checking `docs/cell-index.md` before
authoring, same discipline every prior pack has used); `is_prime_u32` and
`shoelace_area_x2` were deprioritized out of this slice — `is_prime` already covers the full
u16 domain (0..65535), which is most of what AIME primality checks need, and
`shoelace_area_x2` needs genuinely signed intermediate arithmetic (a chained sign-magnitude
computation, not a single checked op) that wasn't worth the design cost for this pass.
Landing these 12 did not run M3 and does not retroactively justify skipping it — the gate's
*reasoning* (precipitation over guessing) stands; this was a one-time, explicitly-authorized
exception to it, not a reversal of the
policy.

## The one-sentence version

Don't build a math runtime to pass a benchmark — run the benchmark through the runtime you
built: plans compile to cells, schemas precipitate into memory, every answer leaves a
verifiable fact behind, and the headline is a small model that stops flinching when the
numbers change.
