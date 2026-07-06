# Crosscheck through the compiler — the M2.6 prediction check (M2.8 item 1)

*2026-07-06 · `crosscheck_m26.py` → `crosscheck_m26_results.txt` · models local via
ollama, temp 0 · registered in `docs/math-campaign-amendment.md` §M2.6/§M2.8.*

## What was run

The pilot's 20-problem cross-check (inline vs library-composed derivation, accept only
on agreement), with one structural change: **every repair the Python harness used to do
now lives in the compiler**. The harness's `autofix()` (macro-strip, paren-collapse) is
deleted; sources go straight into `cell80 compose`, which applies Full canonicalization
(alpha-rename to slots, defer-division, exact constant folding, static width inference),
resolves library calls (search + arity behind a measured confidence floor), runs each
derivation, and applies the registered agreement gate. Extraction (code-fence stripping,
bare-body wrap) stays harness-side — that is transport, not repair.

This was the first time the M2.5/M2.6 passes ever touched model output, and the first
run of the registered M2.6 prediction: **gemma4 yield 80% → ≥90% at unchanged 100%
precision.**

## Results

| model | accepted | precision | yield | escalations (recoverable/genuine) | accepted-wrong |
|---|---|---|---|---|---|
| gemma4:e4b | 15/20 | **15/15 = 100%** | **75%** | 4 / 1 | **0** |
| granite4.1:3b | 7/20 | 7/7 = 100% | 35% | 8 / 5 | 0 |
| qwen2.5:3b | 1/20 | 1/1 = 100% | 5% | 12 / 7 | 0 |

E-code repair tallies: gemma `E0505`×19 (dead lets), `E0301`×1 (auto-widen), `E0203`×1
(trailing-let); granite `E0505`×14, `E0502`×11 (non-straight-line fallbacks), `E0301`×1,
`E0204`×1; qwen `E0505`×15, `E0502`×5, `E0301`×1, `E0202`×1.

## The prediction: FAILED on yield, held on safety — banked as registered

Yield went **down** (80% → 75%), not up to ≥90%. Precision — the half whose movement is
the registered revert trigger — did not move: 23 accepts across all three models, zero
accepted-and-wrong. The passes did not convert a single escalation into a silent wrong
answer, which is the property the whole gate exists to protect.

Why the yield half missed, from the per-row evidence:

1. **Temp-0 outputs drifted between the pilot run and this one**, so the failure set is
   not the failure set the prediction was written against. row93 — the trailing-`let`
   capture the normalizer was built for — came back this run as an *unparseable* source
   (`E0501 parse`), which no normalizer can reach. row94, accepted-correct in the pilot,
   this time emitted an inexact constant division and died with a typed
   `E0302 inexact_const_division` — the fold working as designed on a genuinely wrong
   plan (the alternative was a silently truncated integer reaching the gate).
   Consequence for method: predictions phrased against a specific captured failure set
   are fragile even at temp 0; future predictions should be phrased against failure
   *classes* and verified on captured sources (replay), not fresh generations.
2. **A one-sided recovery cannot clear a two-way gate.** row89's width fix worked — one
   derivation auto-widened (`E0301`) and produced 8000 — but the sibling derivation
   still died, and one valid answer escalates by design. Four of gemma's five
   escalations are *recoverable* (a path had the right answer); all four would accept
   under the registered 2-of-3 rule. **This is the strongest evidence yet that M2.7's
   third derivation is load-bearing, worth up to +20 yield points on this slice.**
3. **granite (H-P2 preview): 35% code-form accepted-and-correct** vs its 30% plan-IR
   combined rate — a gain, but far from the registered ≥2×. Its failure mass is dialect
   (`E0502`×11: method-receiver spellings like `x.method()`, assignment statements,
   parse errors). If the full M2.8 slice confirms, H-P2's kill clause applies: the
   headline configuration changes model, stated openly.
4. **qwen collapsed (1/20) under a systematic `E0501 expected ';'` on 9 rows** — a
   model-specific output dialect this prompt path had not previously surfaced (the
   pilot's 65% qwen figure came from the *bakeoff* path, a different prompt shape, so
   there is no clean baseline for this cell of the matrix). Diagnosis needs captured
   sources, not speculation: possibly one cheap normalizer rule, possibly genuinely
   unparseable output. The M2.7 harness dumps every generated source per row so this
   chase starts from evidence.

## What stands after this run

- The gate's **safety property survived the pipeline migration intact** across three
  models with very different failure dialects — 0 false accepts anywhere.
- The in-compiler repairs demonstrably fire on real model output (`E0203`, `E0301`,
  `E0202`, `E0204` all observed), and the typed E-code tally turns failure-class
  analysis into reading a dict instead of grepping transcripts.
- The registered M2.6 yield number is dead as stated; the *mechanism* it bet on is
  half-proven (width recovery observed, one-sided) and the other half (trailing-let)
  was unfalsifiable this run because the failure it targets didn't recur. Both get
  retested properly under the 3-derivation gate and on captured sources.

## Decisions

1. Build **M2.7** (third derivation, decorrelated reader) next — the recoverable-row
   arithmetic makes it the highest-leverage move on the board.
2. Capture per-row sources in all future runs (replay + diagnosis).
3. Chase the qwen `expected ';'` dialect and granite's `E0502` mass from captured
   sources after M2.7 lands.

---

# M2.7 result — the third derivation, same day (`crosscheck_m27.py`)

*Third reader: deterministic paraphrase-then-extract on the same model (temp 0);
gate: the registered 2-of-3 rule (unanimous / majority-**flagged** / escalate);
every source + paraphrase + compose report dumped under `m27_sources/`.*

| model | yield | strict precision (unanimous) | with majority | accepted-wrong |
|---|---|---|---|---|
| gemma4:e4b | **19/20 = 95%** | 15/15 | 19/19 | 0 |
| granite4.1:3b | 8/20 = 40% | 4/4 | **8/9** | **1 (majority band)** |
| qwen2.5:3b | 3/20 = 15% | 0/0 | 3/3 | 0 |

**gemma4: all four recoverable escalations converted, all correct** — including both
*comprehension* misreads (row86 inclusive/exclusive, row101 "10 more" vs "×11"): the
paraphrase reading broke every tie the right way, which is the decorrelation M2.7 was
registered to buy. The one residual miss (row94) stayed a genuine typed escalation
(`E0302`). On this slice gemma now **exceeds its published GSM8K number (~85%,
stated-answer CoT) at a measured 0% silent-error rate** — N=20, slice runs hot for
gemma (bakeoff 95%), so this is directional de-risking for H-M2/H-P1 at N=1,319,
not the claim itself. M2.7 was worth exactly the predicted +20 points here (75%→95%).

**granite: the H-P1 failure class materialized, in the audit band, and is diagnosed.**
row22 accepted 0 (want 14) as a flagged 2-of-3 majority. The dumped sources show it is
**not a correlated misread**: the two "agreeing" derivations are structurally unrelated
broken programs of the shape `if <expr> == 0 { X } else { 0 }` that both collapse to the
else-arm — *coincidental agreement on failure's default value*, while the one genuine
derivation died on an out-of-dialect method call. Strict-unanimous precision stayed
100%; the flag mechanism did exactly what the two-strictness-level registration exists
for. Two candidate fixes, **both amendments to the registered acceptance rule, so
neither is applied unilaterally**:
- *cheap guard:* majority agreement on the degenerate value 0 does not accept
  (GSM8K answers are essentially never 0; a legit-0 row escalates, which is honest);
- *principled fix:* **literal lifting** — canon already knows every baked literal and
  its slot; promote them to parameters so the counterfactual battery (perturb, keep
  consistent movers) runs on composed cells too. Two broken programs that agree at one
  point will not agree under perturbation. This closes the class structurally and
  reuses machinery `solve` already has.

**qwen: the `expected ';'` mass is diagnosed and is NOT a normalizer gap — it's a
trap.** qwen's dominant dialect is *stated-answer-then-work*: a bare answer literal as
the first body line, followed by (usually broken, often out-of-dialect) derivation
code. The parse error is the stated answer itself. Any "repair" that extracts that
leading literal is stated-answer acceptance through the back door — forbidden by the
`correct_via_solve` discipline on purpose. The 15% yield with 0 wrong is the honest
number for this model under this prompt shape; the recoverable path is prompt-side
(the bakeoff shape it scored 65% under), not compiler-side. Where qwen did emit clean
derivations, the gate accepted them (3/3 correct, all majority).

**Cross-model:** 30 accepts, 1 wrong (flagged band only, diagnosed, class named).
Strict-unanimous precision is 19/19 across all models and both runs to date.
