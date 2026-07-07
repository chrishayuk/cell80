# The gate moves into the compiler — M2.5–M2.7 findings

*2026-07-06 · branch `feat/canonicalization-m25` → `feat/compose-m29`, merged to main ·
raw run logs and per-row captured sources: `experiments/planfix/crosscheck_m26_results.txt`,
`crosscheck_m27_results.txt`, `crosscheck_m27_granite_xreader.txt`, `m27_sources/` ·
running lab notes: `experiments/planfix/crosscheck-m26-findings.md` · registration:
`docs/math-campaign-amendment.md`.*

**TL;DR.** In one day the amended campaign plan's M2.5 (canonicalization), the M2.6
compiler half (constant folding, static width, typed diagnostics), and M2.9's harness
(`cell80 compose` — link loop + N-derivation agreement gate) landed, merged, and were
then run against real models. The registered M2.6 yield prediction **failed and is
banked as a miss** (gemma4 75% vs predicted ≥90% under the 2-way gate). The registered
M2.7 third derivation then delivered exactly what the failure analysis said it would:
**gemma4 19/20 = 95% yield at 100% precision on both strictness levels — above its
~85% published GSM8K stated-answer score, with a measured 0% silent-error rate** —
and a cross-model-reader configuration took granite from 35% to **70% at 100%
precision with zero genuine escalations**. Across every run and model: **49 accepts,
1 wrong — and the 1 was in the flagged audit band, is diagnosed to a named class, and
two independently-verified fixes close it.**

---

## 1. What was built (the compiler now owns the repairs)

- **M2.5 — `rustz80::canon`.** Text→text canonicalization *before hashing* (the
  artifact hash covers a raw-text source hash — AST-only canonicalization would have
  left precipitation unmeasurable; the registered anchor point was corrected for
  this). `Light` mode: the dialect normalizer (macro-strip, trailing-`let`/`return` →
  tail, paren collapse), byte-identical when nothing fires — proven across the whole
  library by an unchanged codegen golden. `Full` mode (compose/campaign path):
  alpha-rename to `q*/v*` slots in dataflow order, topological op ordering, exact
  rational constant folding (decimals become exact fractions: `0.9` ≡ `9/10`),
  defer-division, static width inference, the versioned unit base-scale table.
  All seven registered acceptance tests green (`cell80/tests/canon_acceptance.rs`).
- **M2.6 — typed diagnostics + fold-based width.** Stable `E*` codes with
  `suggested_fix` (`rustz80::diag`); the normalizer is a code→rewrite table; repair
  rows carry codes, so failure-class analysis is a tally, not a grep. `88000/11`
  folds to `8000` at compile time; constants exceeding `u16::MAX` widen the lane;
  inexact constant division is a typed compile error, not a truncated number.
- **M2.9 harness — `cell80 compose`.** Full canon → the `E0504`-cued link loop
  (search + arity behind a measured 0.6 confidence floor; a nonsense call name is a
  typed refusal — the floor was calibrated after a test caught `zorbulate_qq`
  silently resolving to a real cell) → compile → run → the registered agreement gate
  (`unanimous` / `majority`+**flagged** / `escalate`) → `--facts` provenance.
  Composed schemas precipitate: same structure, different nouns ⇒ same artifact,
  second sighting retrieved.
- **Deleted:** the Python harness's `autofix()`. Every deterministic repair now runs
  in the compiler, typed and recorded. Extraction (code fences, bare-body wrap)
  stays harness-side — transport, not repair.

## 2. What was measured

### Run 1 — the 2-way gate (the registered M2.6 prediction check)

| model | yield | precision | wrong | note |
|---|---|---|---|---|
| gemma4:e4b | 75% (15/20) | 100% | 0 | prediction (≥90%) **missed** — banked |
| granite4.1:3b | 35% (7/20) | 100% | 0 | H-P2 preview: +5pts over plan-IR, far from ≥2× |
| qwen2.5:3b | 5% (1/20) | 100% | 0 | systematic parse dialect (see §3) |

The prediction missed for two reasons the per-row evidence makes plain: temp-0
outputs drifted between runs (the failure set was not the pilot's — the trailing-`let`
row came back unparseable; a previously-correct row turned into a genuine
`E0302 inexact_const_division`, the fold refusing a wrong plan), and a one-sided
recovery cannot clear a two-way gate (the width fix recovered one derivation of
row89; the sibling still died). Four of gemma's five escalations were one-path-correct
— which priced the third derivation at up to +20 yield points *before* it was built.

### Run 2 — the registered 2-of-3 gate (M2.7)

| model | yield | strict precision | with majority | wrong |
|---|---|---|---|---|
| gemma4:e4b | **95% (19/20)** | 15/15 | 19/19 | 0 |
| granite4.1:3b | 40% (8/20) | 4/4 | 8/9 | **1 (flagged band)** |
| qwen2.5:3b | 15% (3/20) | 0/0 | 3/3 | 0 |

All four of gemma's recoverable rows converted — **including both comprehension
misreads** (inclusive/exclusive counting; "10 more" read as "×11"): the
paraphrase-then-extract third reading broke every tie correctly. That is the
decorrelation M2.7 was registered to buy, working on the failure class
canonicalization explicitly cannot touch. The one residual gemma miss remains a
*genuine typed escalation* — the gate refusing a wrong plan, which is the property
working.

### Run 3 — the registered second-model reader (granite × gemma-reads)

| granite config | yield | precision (both bands) | wrong | genuine escalations |
|---|---|---|---|---|
| 2-way | 35% | 100% | 0 | 5 |
| 3-way, self-paraphrase | 40% | 100% / 89% | 1 | 4 |
| 3-way, **gemma reader** | **70%** | **100% / 100%** | **0** | **0** |

One cheap inline read by a decorrelated model doubled the weak model's yield, removed
the wrong-accept structurally (a correct third reading breaks degenerate pairings),
and left **zero genuine escalations** — every row in the slice had a correct computed
path under this configuration. Honest framing: this is a two-model ensemble, not
"granite's number"; H-P2 still gets measured granite-only. What it changes is the
campaign's configuration thinking: a weak composer plus a strong reader turns the
gate from a noise filter into an answer finder.

## 3. The error analysis (from captured sources, not speculation)

Per-arm scoreboard (right/wrong/dead of 20):

| model | inline | composed | paraphrase |
|---|---|---|---|
| gemma4 | 19/0/1 | 15/2/3 | 19/0/1 |
| granite | 10/5/5 | 12/4/4 | 5/8/7 |
| qwen | 1/9/10 | **13/7/0** | 4/8/8 |

- **granite's signature dialect is verify-not-compute**:
  `if <arithmetic> == <guessed answer> { guess } else { 0 }`. One habit explains three
  labeled classes: the degenerate-zero wrong-accept (two failed verifications agree
  on the else-arm `0`), the `if…then…else` parse failures, and row89's "width"
  failure reclassified (the arithmetic hid inside a verify-`if`, so Full canon
  soft-fell to Light and the fold never saw it). granite also corrupts its own
  paraphrases (5R/8W — worse than its other arms), which run 3 confirmed by fixing.
- **qwen is one healthy arm surrounded by two dead ones.** Its composed arm produced
  valid code on 20/20 rows and is 65% correct standalone — exactly its bakeoff
  number. The stated-answer-then-work disease (answer literal first, broken "work"
  after — the parse error *is* the stated answer) afflicts only the inline-shaped
  instructions. No honest repair exists: extracting the literal is stated-answer
  acceptance through the back door, which `correct_via_solve` forbids by design.
  Per-model prompt reshaping is the registered no-bespoke-schemas exclusion.
- **Permissiveness, priced** (asked and answered): a documented "Model-Rust"
  superset lowered by canon (then-sugar, method-call sugar, SSA reassignment) would
  preserve every invariant — the oracle tests the canonical form — but the captured
  failures are overwhelmingly *semantic*, not syntactic: the full syntax package is
  worth ~2–3 rows on one model, and `then`-sugar specifically manufactures more
  degenerate-zero surface unless the zero-guard lands first. Dialect entry stays
  priced by escalation tallies, per the existing discipline.

## 4. Amendments — proposed, then registered and verified by replay (2026-07-06)

*Items 1–2 below were **registered** (user sign-off) and implemented: the zero-guard
in `compose::agreement` (`degenerate_zero` gate outcome) and the method-call rewrite
as `E0205 method_to_kernel` in canon. Verified by **replaying the captured sources**
through the amended gate (`replay_m27.py` — no model calls, no drift):*

| config | before | after replay |
|---|---|---|
| granite, self-para | 8 ok / **1 wrong** | **9 ok / 0 wrong** (row22 → `degenerate_zero`; row86 → accept via `E0205`) |
| granite × gemma reader | 14 ok / 0 wrong | 14 ok / 0 wrong; row86 upgrades majority → **unanimous** |
| gemma4 / qwen | 19/20 · 3/20 | unchanged — the amendments cost nothing anywhere |

*Accepted-and-wrong is now 0 in every configuration, measured on the exact sources
that produced the original failure.*

**Literal lifting + the composed-path battery (registered path, 2026-07-07):** canon
`Full` now lifts let-bound literal quantities to parameters (`E0102`; inline
constants stay baked as structure), compose runs cells at their lifted values, and
an accepted multi-derivation agreement must survive the **counterfactual battery**
(each common lifted value perturbed +1, value-keyed across derivations) or it
downgrades to `battery_escalate` — the plan-solve coincidence discipline, now on
composed cells. Replay over the captured sources:

| config | before lifting | after |
|---|---|---|
| gemma4 | 19/20 | **20/20 — 100% yield, 100% precision, zero escalations** |
| granite × gemma reader | 14/20 | **15/20** |
| granite / qwen | 9/20 · 3/20 | unchanged (their failures are pre-runtime) |

row94 — the one gemma miss — was a *false kill*: folding **baked** quantities let
defer-division create an inexact constant intermediate (`E0302`); lifted, the same
structure divides at runtime with the actual values and lands exactly, in all three
derivations. Lifting also makes schemas precipitate across problem *instances*
(same structure, different numbers ⇒ one artifact), not just across nouns — the
strongest H-M3 mechanism yet. The battery killed no legitimate agreement anywhere,
and the ported `a+b == a*b` at `(2,2)` coincidence test dies exactly as it does in
plan-solve.

1. **Zero-guard**: majority agreement on the degenerate value `0` does not accept.
   Counterfactual over every captured report, all models: the single wrong-accept
   disappears at **zero yield cost** (no correct answer was ever a zero-majority).
2. **Numeric method-call → kernel rewrite** (`a.max(b)` → `imax(a,b)`, `.min`,
   `.abs_diff`): deterministic, semantics-preserving, typed; worth 1–2 granite rows.
3. **Literal lifting** (the structural fix behind the zero-guard): canon already
   records every baked literal and slot; promoting them to parameters lets the
   counterfactual battery perturb composed cells — two broken programs that coincide
   at one point will not coincide under perturbation.
4. *(aggressive, needs its own registration and a captured-source precision re-check)*
   **verify-`if` → computed-side rewrite**: `if E == lit { lit } else { 0 }` returns
   `E` — converts granite's signature disease into honest computation.

## 4c. The error-chase backlog lands (2026-07-07): casts, if-value canon, E0207

Three roadmap items from the captured-source analysis, built and replay-verified:

- **Casts** (`as u16`/`as u32`) join the straight-line subset — `as u16` is the
  identity in the narrow lane and a real truncation in the wide one; `as u32`
  commits the wide lane. The granite row22 cast-tail no longer blocks `E0205`.
- **If-value (select) canonicalization** — `if c { a } else { b }` is now a
  canonical node with comparison normalization (`>` flips to `<`; `==`/`!=` sort
  operands) and **lazy-arm emission**: arm-exclusive work renders inline in its arm
  (the guarded-division idiom `if b != 0 { a / b } else { 0 }` keeps its
  kill-avoidance — verified end-to-end at b=0), while condition-reachable and
  both-arm-shared nodes hoist. This brought the *inline* derivation arm — the
  strongest arm — under canonicalization, lifting, and the battery. A positional-ABI
  bug fell out of the guard test before it could ship: dataflow slot assignment was
  reordering real parameter signatures (callers' positional args silently remapped);
  real params now keep positional slots, only lifted quantities get dataflow order.
- **`E0207 verify_rewrite`** (registered amendment 3): `if E == lit { lit } else
  { 0 }` returns the computed side. Replay precision check across every
  configuration: accepted-wrong stayed 0 — the rewrite stands. On this slice it
  converts nothing (granite's real verify-ifs are `then`-syntax or panic-arm
  variants that die earlier); it is armed for M3-scale sources.

Replay after all three: gemma **20/20 with two rows upgraded to unanimous** (18/20
strict); granite/qwen unchanged; and one genuinely important downgrade —
**granite-xreader row117 went accepted-correct → `battery_escalate`, and the battery
is right**: its two agreeing derivations share the reading (7:13 of 120) but one
truncates early (`120/20` then `*7`), agreeing with the deferred form *only because
120/20 divides exactly*. Under perturbation they diverge (45 vs 40). The agreement
was exact-division coincidence — precisely the fragility GSM-Symbolic perturbs and
H-M1 measures, caught in the wild on real model output. One yield point spent on
the guarantee that accepted agreements generalize off-instance.

## 4b. The PAL baseline (H-M2) and width routing — 2026-07-07

**PAL-Python** (`pal_baseline.py`, one derivation, subprocess exec, no gate — every
wrong answer silent by construction):

| model | PAL accuracy | PAL silent-wrong (+crash) | cells yield @ 0 silent |
|---|---|---|---|
| gemma4 | 90% | 2 (+0) | **100%** (3-way + lifting + battery) |
| granite | 75% | 3 (+2) | 45% solo · 75% ensemble |
| qwen | 70% | 5 (+1) | 15% |

**H-M2 verdict on the 20-slice:** *passes decisively for gemma* — cells beat PAL by
ten points while eliminating its silent errors. granite reaches PAL parity only in
the ensemble configuration (equal accuracy, 0 silent vs 3 silent + 2 crashes). qwen
fails H-M2 solo — its Python is fine and its restricted-Rust inline arm is not, which
localizes the gap to instruction shape, not the substrate. Across all 60 PAL rows: 10
silent wrongs (16.7% of answers indistinguishable from correct ones); across every
cells configuration ever run: 0. The registered claim that the differentiators are
precision and auditability, not raw accuracy, is now measured, not asserted.

**Width-aware routing** (`CellHost::search`): width-intent queries (exact tokens
`wide/u32/32-bit/65535/large/big/huge` — superlatives excluded) stably rank wide
cells ahead of u16 siblings, order-only, no score rescaling. Direct p@1: 0.7934 →
**0.8413** at 263 cells; paraphrase also up (0.33 → 0.37). The retrieval floor is
restored to 0.80.

## 4d. M2.8 closes (2026-07-07): parity, a silent-wrap hole found and fixed, and the second ensemble datapoint

- **Cross-language defer-division parity: 7/7 PASS** (`parity_check.py` →
  `parity_check_results.txt`). The registered byte-parity wording predates the fold
  (canon reduces `30/100`→`3/10`; the Python arm deliberately doesn't) and the shape
  split, so the meaningful invariants are what's checked: numeric equivalence on the
  cell VM vs the simulated Python plan, and one-trailing-div structure on both arms.
  Canon strictly subsumes `equations_to_plan` normalization.
- **The parity check caught a real precision hole before M3 could ship it.**
  Lifting made quantities non-constant, so the fold could no longer see that
  `q0 * 1000` overflows at the source's own values: `88*1000/11` silently wrapped
  to 2042 in the narrow lane — and identical schemas wrap *identically*, so a gate
  could have agreed on the wrapped value across derivations. Fix (**checked
  emission**, `CanonOptions::checked`, on for the whole compose path): lifted cells
  emit adds/subs/muls through the checked prelude kernels — overflow and negative
  intermediates escalate, never wrap — matching the plan renderer's semantics.
  Full replay after: every configuration unchanged at 0 accepted-wrong; parity 7/7.
- **qwen × gemma-reader: 15% → 65% at 100% precision, 0 wrong, 0 genuine
  escalations** — the ensemble treatment reproduces on the second weak model,
  landing exactly on qwen's composed-arm ceiling, as it did for granite (35%→75%).
  The weak-composer + strong-reader configuration is now evidenced twice.

## 4e. Width belongs to the compiler + the last pre-M3 nits (2026-07-07)

Registered amendments 4–5, from the observation that suffix errors are bookkeeping
noise, not arithmetic intent:
- **`E0208 suffix_normalized`** — integer suffixes are advisory in Full mode
  (stripped on parse, canonical ones re-emitted by the lane); an impossible
  `88000u16` is named in the repair row and the *value* decides the lane.
  All three spellings of a constant now reach one schema.
- **`E0209 narrowing_dropped`** — a model's mid-chain `as u16` drops in the
  checked lane (it fights the type checker; the kernels own overflow). Plain
  Full/Light keep real truncation — the dialect and its rustc oracle untouched.
  Replay precision check: every configuration still 0 accepted-wrong.

Plus two pre-M3 fixes from the residual-error analysis: **exact-id linking**
(`eq(a, b)` links by manifest lookup before any fuzzy score — 2-char names were
unlinkable below every threshold) and **cost fields in the compose report**
(`cycles`, `trapped_ops` per derivation — H-M4's cost-per-verified-answer needs
them from day one). Remaining pre-registered-for-M3-runner: per-generation
provenance (model digest, seed, options) lives in the campaign runner when it's
built — drift already killed one prediction, so results must be replayable.

## 4f. Second residual-error pass (2026-07-07, post-amendments)

Regenerating per-row outcomes across all five configurations after amendments 1–5:

- **E0205 was width-blind — found and fixed.** The method rewrite targeted `imax`,
  a u16 kernel, which can't take checked-lane values (granite row22 d1:
  "argument 1 of `imax` is 16-bit"). The prelude gains `imax_u32`/`imin_u32`/
  `iabs_diff_u32` (free fns, DCE-pruned) and the wide lane emits the `_u32` forms
  with wide arguments.
- **SSA reassignment landed** (the row92 class): `total = total + n` accumulator
  style rebinds like a `let` shadow — semantics-preserving in a straight-line
  body; `let mut x = a; x = x + b; x * 2` and `(a + b) * 2` are now one schema.
- **Revived arms fail safely.** The suffix/checked work resurrected previously-dead
  *wrong* derivations (granite row89 d1 now computes 79200) — every one lands as a
  disagreeing answer and escalates. Zero wrong accepts held through two rounds of
  coverage expansion; the gate is robust to arms coming back to life wrong.
- **What remains, honestly:** qwen's stated-answer parse mass (20 of 34 kill rows —
  instruction-shape, ensemble-solved); granite comprehension disagreements
  (row121: three valid answers, all different — the gate's job); two tally-gated
  Model-Rust one-offs (chained comparison `a < b < c`, exotic call targets); the
  verify-if-with-panic-arm shape (row89 d0 — its answer exists only as a stated
  literal, unrecoverable by design); and stranded single-correct-answers in
  ensemble configs, which is the registered 4th-derivation question for M3 data.

## 4d. `then`-sugar desugaring (E0210) — the verify-rewrite's missing feeder (2026-07-07)

E0207 (§4c) rewrites the verify-not-compute shape `if E == lit { lit } else { 0 }` to
its computed side, but replay showed it *converted nothing* on granite's captured
slice: granite writes that shape in **non-Rust `then`/`else` sugar**
(`if (42 * 10) / 3 == 140 then 140 else 0`), which dies at `E0501 parse` before any
AST pass runs. E0210 is the feeder — a comment-safe textual pre-pass (in
`canon::canonicalize_source`, before `syn::parse_str`) that desugars
`if C then a else b` → `if C { a } else { b }`, coercing a `!`/`panic!()` else-arm to
`0`. `then` never appears in valid Rust, so the pass is byte-identical on any
well-formed source; only the code portion of a line is considered, so a `then` in a
`//` comment (gemma row94 d1) is left alone — verified by the codegen golden staying
green.

**Replay across all five configs (captured sources, no model calls):**

| config | before | after | accepted-wrong |
|---|---|---|---|
| gemma4 | 20/20 | 20/20 | 0 |
| granite solo | 9/20 | **10/20** | 0 |
| granite × gemma-reader | 14/20 | 14/20 | 0 |
| qwen solo | 3/20 | 3/20 | 0 |
| qwen × reader | 13/20 | 13/20 | 0 |

granite's **row104** recovers (`escalate → 140/majority`): its `then`-sugared arm now
parses, and because its guess (`140`) *matched* the arithmetic `(42*10)/3`, the
constant-condition fold evaluates it correctly and pairs with the already-correct
`max(0, …)` arm (granite_xreader tightens the same row majority → unanimous).
**row111 does NOT recover, and that is the honest outcome**: its guess (`43`) differs
from the true `(2*24+3*14)/2 = 45`, so the const-fold returns the coerced else `0` →
`degenerate_zero`, which the zero-guard refuses. (E0207 itself only fires when the
computed side is non-constant; granite's all-literal verify-ifs take the const-fold
path instead.) Precision held at **0 accepted-wrong in every configuration** — the
pass only ever turns a parse-dead arm into a valued one, and any false agreement is
caught by the zero-guard and the counterfactual battery.

## 5. Honest limits

N=20, and the slice runs hot for gemma (its no-gate bakeoff was already 95% here vs
~85% published on the 1,319-row test set) — today's headline is directional
de-risking for H-M2/H-P1, not the claim; the claim is made at N=1,319 in M3 under
the pre-registered accounting. Temp-0 generation drift is real and already bit one
registered prediction: future predictions get phrased against failure *classes* and
verified by replay on captured sources (which all runs now dump). The cross-reader
number is an ensemble configuration and is labeled as such. The M2.8 remainder —
PAL-Python baseline (H-M2), granite-only H-P2 measurement, Python-arm defer-division
parity by replay, the retrieval-curve re-measure — is still owed before M3, and the
zero-guard/method-rule amendments await explicit registration.
