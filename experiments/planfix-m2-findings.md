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

## 4. Proposed amendments (registered rule changes — not applied unilaterally)

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
