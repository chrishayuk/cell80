# CN-7: Numeracy Midtrain with Cells as Validator

**Pre-registration v0.1 — Predictions pinned before any training**

Chris Hay | CN Programme | July 2026

---

## 1. Purpose

Test whether the CN-6 generation arm — dead at resolve@5 0.042 on v11 and
unrescued by a math-capable 1B base — can be revived by installing exactly
one thing: arithmetic competence *inside the emission grammar*, via a
numeracy midtrain of the v11 checkpoint in which the cell library authors
the curriculum, injects all beyond-tier answers under a loss mask, and
verifies every number.

Two theses are on trial, both falsifiable:

1. **The bottleneck thesis.** CN-6's generation failure is
   arithmetic-under-emission-format, not arithmetic per se. Evidence for:
   Llama-3.2-1B passed the 10/10 easy-arith probe yet reached only 0.306
   correctness in-format, with resolution pinned at the floor. Llama was
   never trained on the joint skill; the midtrain trains exactly it.
2. **The division-of-labour thesis.** A model should hold basic math
   natively and rely on cells for everything beyond, with the boundary
   enforced at the gradient level: beyond-tier answers are
   environment-injected and loss-masked, so no gradient ever trains them
   into weights. The call is the abstention.

The experiment is designed so that **failure is publishable**. If emission
correctness reaches its target and resolution still does not move,
generation is computation-limited in a way targeted training cannot fix at
~100M, the Llama result generalises, and delegate-by-pointing /
delegate-by-carrying stands as the final answer.

---

## 2. Pinned baselines

Every prediction below grades against a number that already exists. No
baseline may be re-measured after the midtrain except on the identical
protocol.

| # | Quantity | Value | Provenance |
|---|---|---|---|
| B1 | Generation resolve@5, held-out n=24, v11 | 0.042 | CN-6 stage 2 |
| B2 | Generation per-pair correctness, v11 | 0.097 | CN-6 stage 2 |
| B3 | Generation correctness, Llama-3.2-1B swap | 0.306 | CN-6 base swap |
| B4 | Generation resolve@5, Llama-3.2-1B swap | ~0.08 (CI ≤ 0.31) | CN-6 base swap |
| B5 | Oracle router ceiling, 6 examples, 249 value cells | 0.62 P@1 / 0.83 P@5 | CN-6 powered LOO |
| B6 | Router degradation, 1 of 6 examples wrong | 0.71–0.79 | CN-6 noise sweep |
| B7 | Extraction resolve@5, held-out n=24 | 0.875 [0.69, 0.96] | CN-6 stage 2 |
| B8 | Extraction example correctness | 0.979 | CN-6 stage 2 |
| B9 | Fingerprint held-out median rank, v11 seed 81, n=200 random | 98 (p75 227, 38% top-10%) | CN-1 faithful number |
| B10 | Fingerprint seed-invariance | rank 43/44/44 across seeds, std 0.47 (nulls: std 77–91) | CN-1 third dissociation |
| B11 | Held-out cells at generation correctness 0.00 | 12/24 (jacobi, crc16, isqrt, mobius, …) | CN-6 postmortem |
| B12 | Paraphrase cliff precedent | canonical 10/10 → narrative 0/10 | v11 KnnStore |
| B13 | Cell execution cost | microseconds per call; full 790-cell brute force ~1 ms | runtime |

---

## 3. Definitions

### 3.1 Tier frontier

**Tier A (in-weights numeracy)**, defined *negatively* by the cell
library — teach only what sits below the cheapest cell:

- single- and double-digit addition/subtraction
- small multiplication (up to 2-digit × 1-digit; times tables)
- integer comparison and ordering
- parity; small modulus; counting/successor

Explicitly excluded: multi-digit multiplication, anything
digit-manipulation-flavoured (checksums, digit reversal), modular
exponentiation, number-theoretic predicates. Under-teach on principle: the
tier exists to make small instances computable, not to blur the boundary.

**Within-frontier cell**: a held-out cell whose *small instances* reduce to
Tier A operations (e.g. small gcd by repeated subtraction is borderline —
classify each of the 24 held-out cells before the midtrain and freeze the
list; the classification is part of this pre-registration's artifacts).

**Beyond-frontier cell**: everything else. Expected to include most or all
of the 12 cells at correctness 0.00 (B11).

### 3.2 Data species (all cell-authored, all cell-signed)

| Species | Content | Loss |
|---|---|---|
| S1 — Tier A drill | in-tier arithmetic, 50:50 canonical ("7+5=12") : narrative-embedded (TinyStories register) | full loss incl. answers |
| S2 — Tiny-GSM interleaved | TinyStories-register word problems; per step, in-tier → model computes (answer in loss); beyond-tier → cell call emitted, result injected and **masked** | loss on text + call + continuation; **zero loss on injected results** |
| S3 — Emission transcripts | CN-6 grammar: k=6 oracle-correct I/O pairs per cell, deliberately varied/discriminative inputs | full loss |
| S4 — Replay | TinyStories, 30–50% of the mix | full loss |

Cells run at **corpus-build time**: results baked in, mask spans recorded.
Live execution is reserved for the eval harness and CN-7.6.

### 3.3 The mask

Per-token loss mask over environment-injected spans in S2. Property to be
audited, not assumed: **no beyond-tier answer token anywhere in the corpus
carries loss.**

---

## 4. Experiments

Order is cheap-first. The decision spine is 7.1 → 7.2 → 7.3; 7.0 and 7.4
are free measurements; 7.5 is the control that makes the result
defensible; 7.6 is conditional.

### CN-7.0 — Yield curve (instrumentation; one evening; no training)

**Protocol.** Sample current v11's emissions across the tier frontier
(same prompting as CN-6 stage 2). Cell-sign every pair. Report signed-pair
yield per tier and per cell.

**Predictions.** Tier A yield low but nonzero (0.05–0.15 per-pair,
consistent with B2); beyond-tier ≤ 0.01.

**Role.** (a) Pre-midtrain baseline on the identical protocol for every
later metric. (b) Decides cold-start vs bootstrap for CN-7.6. No gate.

### CN-7.1 — Corpus build + audits (data only; gate before any GPU time)

**Protocol.** Generate S1–S3 per §3.2. Then two consistency checks:

1. **Signature audit**: re-execute every answer token in the corpus
   against its cell. Target: 100% signed. Any mismatch is a pipeline bug.
2. **Mask audit**: verify zero beyond-tier answer tokens carry loss, by an
   independent code path from the one that wrote the masks.

**Rationale.** All four catches in the CN-1 lane were "two routes to the
same number disagree" (untied head, dropped norm, winner's curse, first-N
sampling). This lane builds the two-routes check into the pipeline instead
of discovering it post hoc.

**Gate.** Both audits clean, or no training happens.

### CN-7.2 — Midtrain + regression panel (the spend)

**Protocol.** Midtrain the pretrained v11 checkpoint on the S1–S4 mix
(10–20M tokens against v11's ~100M pretrain; hours on MPS). Full-model
update, replay ratio 30–50%. Then re-run the fingerprint-arm cell finetune
*identically* (792-token vocab extension, seed-81 protocol; 3 seeds if
budget allows — seed-invariance (B10) is the strongest dissociation and
worth re-confirming).

**Pre-registered panel** (all must pass to proceed to 7.3):

| Metric | Threshold | Grades against |
|---|---|---|
| P-a1 | In-tier probe correctness, canonical | ≥ 0.90 | — |
| P-a2 | In-tier probe correctness, narrative-embedded | ≥ 0.80 | B12 (the paraphrase slice is a **gate**, not a nice-to-have) |
| P-b | **Emission-format per-pair correctness, within-frontier** | **≥ 0.83** | B6 (router's graceful zone) — the load-bearing number |
| P-c | Extraction resolve@5, held-out | ≥ 0.79 (within B7's CI) | B7 |
| P-d1 | Fingerprint held-out median rank (seed-81 protocol) | ≤ 130 | B9 (~98 + noise allowance) |
| P-d2 | Fingerprint seed std (if 3 seeds run) | ≤ 10 | B10 (0.47 vs null 77–91) |
| P-e | TinyStories replay loss | ≤ +5% vs pre-midtrain | v10a-era eval |

**Kill criterion.** If P-c or P-d1 regress materially, stop; fall back to
the attention-only variant (FFN frozen, v10a-style) before touching 7.3. A
midtrain that buys numeracy by selling the two working delegation paths is
a net loss. Note the tension going in: fluency provably lives in attention
on this architecture (v10a), but arithmetic circuits in small transformers
tend to want FFN capacity — full-model-with-replay is the primary arm for
that reason, attention-only the fallback.

### CN-7.3 — Generation re-run, stratified (the headline)

**Protocol.** Exact CN-6 stage-2 protocol, held-out n=24, stratified by
the frozen within/beyond-frontier list from §3.1. Wilson CIs; n is small
and the split smaller — report per-stratum counts, not just rates.

**Predictions.**

| Stratum | Metric | Prediction | Grades against |
|---|---|---|---|
| Within-frontier | resolve@5 | ≥ 0.50 (strong: ≥ 0.70), climbing from ~0.08 toward the 0.83 ceiling | B1, B4, B5 |
| Beyond-frontier | resolve@5 | ≤ 0.15 (stays at floor) | B1, B11 |
| Within-frontier | per-pair correctness | ≥ 0.83 (re-confirms P-b in the live protocol) | B2, B6 |

**Reading the outcomes.**
- Both strata as predicted → the division-of-labour claim measured
  directly; generation joins pointing and carrying as a working path,
  *inside the frontier only*.
- Within-frontier fails **with P-b passed** → the informative null:
  emission-correct arithmetic still doesn't resolve; computation-limited
  in a way training can't fix at ~100M; the Llama result generalises;
  pointing/carrying is the final answer. Publishable either way.
- Beyond-frontier climbs → the boundary leaked; go to 7.4/7.5 to find the
  leak before believing it.
- The uninformative outcome is running 7.3 with P-b failed — hence the 7.2
  gate.

Known residue not addressed by numeracy: the correct-but-non-discriminating
tail (luhn/mobius: correct pairs, rank 49/20). That is an input-selection
behaviour and lives in S3's varied-inputs training; report it as its own
line, do not fold it into the stratum rates.

### CN-7.4 — Mask-leak probe (free interpretability)

**Protocol.** Post-midtrain, no cell access: probe beyond-tier arithmetic
directly. On S2-style interleaved problems: measure call-rate on
beyond-tier steps and unassisted-attempt rate.

**Predictions.**

| Metric | Prediction |
|---|---|
| Beyond-tier correctness, no cell access | ≤ 0.05 — despite thousands of masked exposures |
| Call-rate on beyond-tier steps | ≥ 0.95 |
| Unassisted attempts on beyond-tier steps | ≤ 0.05 |

**If it leaks** (beyond-tier competence entering through the reasoning
text around the masks): a genuine finding about where arithmetic forms —
feed it to the interpretability track. Either branch is a result.

### CN-7.5 — No-mask control (the ablation a reviewer will demand)

**Protocol.** Identical corpus, masks removed: beyond-tier injected
results now carry loss. Same finetune, same panel, same 7.3 and 7.4
measurements.

**Predictions.** (a) 7.4's beyond-tier probe goes nonzero via memorisation
of *trained* instances but does not generalise off the training
distribution; (b) the paraphrase slice (P-a2) pays for the extra
memorisation; (c) within-frontier resolution is not improved over the
masked arm. This isolates what the mask buys. Secondary ablations (Tier C
abstention slice on/off; replay ratio) only if 7.3 is ambiguous.

### CN-7.6 — STaR loop (conditional)

**Gate.** Re-run 7.0 on the midtrained model. Activate only if Tier A
signed yield ≥ 0.30 (sampling cost is the loop's only real cost — cells
verify at ~10⁻⁵ of step cost (B13), so verification is free and the model
is the bottleneck).

**Protocol.** Live cell verification in the loop: model emits, cells sign
or reject; signed emissions recycle into training data; rejected
beyond-tier emissions are harvested as boundary supervision (free Tier C).
Loss-level masking of unsigned pairs — zero gradient credit for a wrong
emission, which directly attacks the plausible-wrong failure mode (B6:
off-by-one pairs poison the router hardest).

**If yield stays low.** The loop waits; nothing above it blocks.

---

## 5. Decision spine

```
7.0 yield curve ──────────────► baseline + 7.6 gate input
7.1 audits ── clean? ── no ──► fix pipeline, do not train
        │ yes
7.2 midtrain + panel ── P-c/P-d regress? ── yes ──► attention-only fallback, re-panel
        │ pass
7.3 stratified generation ──► headline result (either branch publishable)
        │
7.4 mask-leak probe (free)     7.5 no-mask control (defensibility)
        │
7.6 STaR loop (yield-gated)
```

---

## 6. Budget

| Item | Cost |
|---|---|
| 7.0 yield curve | one evening, sampling only |
| 7.1 corpus + audits | data pipeline; no GPU training |
| 7.2 midtrain | 10–20M tokens, hours on MPS; + finetune re-run (×3 if seeded) |
| 7.3 / 7.4 | eval passes; cell verification ~free (B13) |
| 7.5 | second midtrain + finetune, same scale as 7.2 |
| 7.6 | sampling-dominated; activate only past the yield gate |

---

## 7. What this pre-registration commits to

1. The within/beyond-frontier classification of the 24 held-out cells is
   frozen **before** the midtrain and shipped as an artifact of 7.1.
2. No threshold in §4 moves after 7.1's audits pass.
3. Negative results ship: the (P-b passed, 7.3 failed) branch is written
   up with the same care as the positive branch.
4. Every number reported has two routes to it where feasible (signature
   audit, mask audit, per-stratum counts alongside rates) — the CN-1
   lesson, institutionalised.
