# Behaviour as a Tool Address — CN-1 results

*A model can be given behaviour-derived embeddings for a library of executable cells, such that a
cell it has never seen invoked still has a usable address, computed from what the cell does rather
than what it is called. This document is the complete, honest write-up of the CN-1 experiment: what
was tested, what held, what deflated under scrutiny, and what remains open.*

**Status: 2026-07-14, final for this arc.** Frozen design + all amendments:
`cell-native-architectures-cn1-preregistration.md`. Running lab record:
`cell-native-architectures-findings.md`. Scripts and result JSONs: `cell-native-architectures/`.

---

## Abstract

We test whether *executed behaviour* can serve as a language model's address for a tool, in a
library of 790 verified "cells" (small, exactly-executable programs). Each cell token's embedding is
`W_f(fingerprint)` — a shared projection of the cell's behaviour on a fixed probe battery, used as
both the input embedding and the tied output-head row, so a held-out cell is both readable and
emittable. Against two controls (a **shuffled**-fingerprint arm that keeps the projection but
scrambles the behaviour↔cell map, and a **random** free-embedding arm), behaviour-derived embeddings
give never-invoked cells an address the controls cannot: on held-out cells the fingerprint arm
reaches median rank **98/790** (38% in the top 10%) versus the controls' worse-than-chance ~500. The
mechanism is confirmed three independent ways — a **double dissociation** (fingerprint is *worse* on
seen cells, far *better* on held-out), a pre-registered prediction that **replicated 3/3 seeds and
across two base models**, and near-zero **seed-variance** (the address is computed, not learned). The
usable operating range, derived from execution economics, is **hundreds of thousands of cells today**
(~10⁵). Whether it reaches 10⁶ is governed by a scaling exponent we measured with two independent
methods that agree: **α ≈ 0.65** (retrained curve 0.62 [0.38, 0.87]; synthetic-expansion curve to
10⁴ cells 0.67 [0.53, 0.82]) — **sublinear with confidence** (the address gets relatively stronger as
the library grows), but near the edge of the threshold for a standalone 10⁶ address, which therefore
routes to the two-tier design. Several headline numbers deflated under scrutiny; the mechanism did
not.

---

## 1. The question, and the gap it sits in

**Not novel:** representing tools as vocabulary tokens is established (ToolkenGPT learns a per-tool
"toolken" embedding; ToolGen makes each tool a unique token). We do not claim it.

**The gap:** two literatures have never been connected.
- *Tool-learning* solves generalization to **unseen** tools exclusively through **language** —
  documentation/description comprehension (GenTool, TOOLVERIFIER, Re-Invoke, RaTA-Tool; the state of
  the art, CoTools, selects from natural-language tool descriptions; Tool2Vec is usage/query-derived,
  still language). ToolkenGPT itself "cannot use unseen tools without retraining" and shows "a strong
  bias toward a small subset of tools it had memorized" — a failure our **random** arm reproduces.
- *Program-embedding* learns representations from **execution traces** (DYPRO, LiGer, Trex, sem2vec),
  precisely because syntactically-similar programs behave differently — but only for program
  *analysis*, never as an LLM's token embedding.

Our conjunction — **executed behaviour as the tool token's zero-shot address** — is the unoccupied
intersection. Its precondition is the ability to execute the whole library exhaustively and cheaply
to compute every address; that is what the cell80 GPU interpreter (~3.7×10⁸ evals/s) buys, and it is
what makes the experiment possible. The paper's spine is therefore **behaviour vs. language as a tool
address**, and the relevant comparison at deploy time is against description-routing (CoTools), a
strong baseline we build but leave for the next arc.

## 2. Method

**Model.** TinyModel v11 (115M, PyTorch, weight-tying native) and — for base-independence —
SmolLM2-135M (code/math-pretrained, tying verified). Vocabulary extended by ~790 atomic cell tokens
+ call delimiters, appended so trained rows are byte-preserved.

**Three-way tying (the core).** Each cell token's row is `W_f(fingerprint)` — one shared MLP over the
20-probe behavioural fingerprint (encoded 40-d: values + a "ran-cleanly" mask), used as *both* input
embedding and tied output head. A held-out cell's row is `W_f` applied to its fingerprint, frozen —
computed, never trained. Constrained decoding masks to the cell-token set (held-out cells included),
so the mask measures *selection*, not vocabulary membership.

**Arms.** (c) **fingerprint** = `W_f(behaviour)`; (s) **shuffled** = `W_f` over a fixed derangement
of the fingerprints (same projection, scrambled behaviour↔cell map — isolates behaviour from "any
structured projection"); (b) **random** = free learned rows (the ablation). All arms share the corpus
and every non-cell weight.

**Corpus.** No H1 factory existed; we built one. Each training row grounds a call in a compositional
operation *descriptor* (the smoke slice showed behaviour-only grounding is unlearnable at this
scale — the frozen base gives no operation signal at the call site). Two factorized held-out axes:
cells (axis A, ~10% stratified, never invoked in training) and template×pack compositions (axis B).
Every result is oracle-verified.

## 3. Results

### 3.1 The mechanism: a double dissociation

Authoritative, faithful, random-sampled (v11, seed 81), held-out cells (novel-cell × seen-composition,
n=200), median rank of 790:

| arm | held-out rank | held-out top-10% | seen rank (control) |
|---|---|---|---|
| **fingerprint** | **98** | **0.38** | 29 |
| shuffled | 498 | 0.045 | 25 |
| random | 539 | 0.0 | 2 |

Two facts, jointly decisive:
1. **Held-out transfer is behavioural.** Scrambling the behaviour↔cell map (shuffled) collapses
   held-out ranking to *worse than chance* (498), alongside random (539). So the address is the
   fingerprint↔behaviour correspondence — not the projection layer, not name-similarity.
2. **A double dissociation.** On *seen* cells the ordering **inverts**: fingerprint is *worst*
   (median 29), random *best* (2), along a "freedom to memorize" axis. The skeptic's default
   ("fingerprint is just a better initialization") predicts fingerprint ≥ shuffled *everywhere*; the
   seen-cell inversion is the opposite. The mechanism the hypothesis names — behavioural geometry
   constrains similar cells to similar rows, costing seen-cell precision and buying an unseen-cell
   address — is what remains.

### 3.2 Robustness: three independent confirmations

- **Pre-registered prediction, replicated 3/3 seeds.** Before running seeds 81/82, we registered:
  fingerprint underperforms shuffled on seen top-1 *and* outperforms on held-out rank, in every seed.
  It held in all three (fingerprint seen-top1 0.27/0.295/0.285, held-out rank 43/44/44; shuffled and
  random both worse on held-out in every seed). A forecast that could have failed, did not.
- **Base-independence.** The SmolLM2 swap reproduces the whole dissociation (seen top-1 fp 0.36 <
  shuf 0.52 < rand 1.0; held-out rank fp 20 ≪ shuf 272 < rand 404) — not a TinyStories artifact.
- **Seed-invariance (a third, free dissociation).** Held-out rank across seeds: fingerprint
  **43/44/44 (std 0.47)** vs shuffled (std 91) and random (std 77) — **~193× less seed-variance.**
  Only the mechanism predicts it: the fingerprint row is a deterministic function of behaviour,
  identical regardless of init, while a random/scrambled row *is* the seed. Near-zero variance is the
  geometry showing through.

### 3.3 What the address resolves to

The cells the model ranks above the true held-out cell are **behaviourally related** to it — mean
fingerprint agreement ~2.7× chance (median), same-family-enriched — but "related," not "identical":
the confusions sit at ~0.44 agreement, and a held-out cell has a **median of 0** genuine
near-duplicates (agreement ≥ 0.8). So the plateau is not a structural rank-1 ceiling; the true cell
is genuinely distinct and rank-1 is not forbidden. A probe-richness sweep found the 20-probe battery
over-estimates similarity only modestly (+0.033 on random pairs, most of the apparent effect being
winner's curse), so a richer fingerprint would sharpen the address only a little — capacity/corpus,
not probe coarseness, dominates the current level.

### 3.4 Usability, derived from execution economics

Execution is near-free, so the deploy question is not top-1 but: *is the true cell within the
candidates executable per token, K_exec?* At 117 tok/s and a CN-2-style 4.8% overhead budget,
**K_exec ≈ 4,718 (GPU) and ≈ 13 (CPU)**. Per-**cell** recall (the unit "can a new cell be found"),
faithful seed-81:

| budget | K_exec | per-cell recall |
|---|---|---|
| CPU @ 4.8% | 13 | 0.125 |
| CPU @ 100% (2× latency) | 266 | 0.833 |
| **GPU @ 4.8%** | **4,718** | **1.000** |

**Usable on GPU at today's scale** (every new cell found within a G2-overhead budget); **CPU only at
a large latency premium.** GPU-in-the-loop is a real architectural dependency, not a footnote.

### 3.5 Scaling: measured, not yet pinned

The address's value is scale-invariance (at 790 cells you could brute-force the library), so the
governing question is how held-out rank grows with library size. Retraining the address at six
library sizes (114→790, holding axis-A cells in, `W_f`-only retrain on the frozen trained transformer
— validated to reproduce the full-model rank, 96 vs ~98):

| N | held-out rank | chance (N/2) | lift |
|---|---|---|---|
| 114 | 34 | 57 | 1.7× |
| 175 | 32 | 88 | 2.7× |
| 270 | 45 | 135 | 3.0× |
| 415 | 76 | 208 | 2.7× |
| 640 | 87 | 320 | 3.7× |
| 790 | 96 | 395 | 4.1× |

Log-log fit `rank(N) = 98·(N/790)^α`: **α = 0.624, SE 0.088, 95% CI [0.38, 0.87].** With six points
over less than a decade, the pre-registered threshold (α < 0.54, for rank(10⁶) < GPU K_exec) sits
*inside* the interval — this curve alone is **underpowered** (the non-monotonic N=175 point is the
noise floor). So we extended it.

**A second decade, a second method.** We grew the library to 10⁴ cells with 9,210 *density-matched*
synthetic cells (fingerprint clones with 30% of probes resampled from per-probe marginals — synthetic
nearest-real agreement median 0.70 ≈ the real library's, slightly denser, i.e. conservative), and
measured held-out rank among N with the **fixed** seed-81 `W_f` (a different method — the deployment
model of "a library grows with synthesized cells the model was not retrained on"; N=790 reproduces
the retrained rank, 86 vs ~96, validating the path):

| N | rank | lift | N | rank | lift |
|---|---|---|---|---|---|
| 790 | 86 | 4.6× | 5,000 | 245 | 10.2× |
| 1,500 | 113 | 6.6× | 8,000 | 386 | 10.4× |
| 3,000 | 162 | 9.3× | 10,000 | 462 | 10.8× |

**α = 0.673, SE 0.053, 95% CI [0.53, 0.82]** — and it **agrees with the retrained curve's 0.624**.
Two methods (retrain-per-N; fixed-`W_f` with structured distractors) converging on ~0.65 is the
cross-check that makes the exponent trustworthy, and the extra decade cut SE from 0.088 to 0.053.

What this establishes:
- **Sublinear, with confidence** (upper CI 0.82 < 1); **lift over chance grows monotonically
  1.7×→10.8×** across the full 114→10⁴ range — the address gets relatively stronger at every scale.
- **The verdict shifted from "undecided" to "leaning against a standalone 10⁶":** the threshold
  α < 0.54 now sits at the *lower edge* of the CI (0.53); extrapolated rank(10⁶) ≈ 8.5k–10.5k vs GPU
  K_exec 4,718 (~2× over). Not a clean fail — 0.53 barely passes — but no longer symmetric.
- **Usable envelope (GPU @ 4.8%): ~3×10⁵–4×10⁵ cells** on both methods — hundreds of thousands, short
  of millions for a *standalone* address.
- **No softmax cliff** in the fixed-`W_f` regime — rank grows as a smooth power law straight through
  the ~2,500-candidate mark. But this does **not** test the *learned*-routing bottleneck (the address
  here is fixed, not a softmax retrained over 10⁴ tokens); that remains owed (§5).

## 4. Limitations, and the correction trail

Six load-bearing numbers were wrong at some point in this work, and every one was caught by the same
method — *compute the number by two routes; treat a disagreement as the finding* — not by a failing
test. In order: the pilot's **untied head** (silent 0.000; arm-vs-arm signature); a **dropped norm**
in checkpointing (reload-vs-training-time rank); a **winner's curse** inflating an over-merge claim
(selected-vs-random pairs); a **first-N sampling** bug that inflated held-out median rank (21→~114 →
authoritative 98) and confusion enrichment (6.73×→~2.7×) because the eval read a cell-grouped file;
a **null-population** mismatch (0.065 all-cells vs 0.163 same-arity); and an **unsupported FAIL** verdict
on the scaling curve (point estimate reported without its CI — the same error as an unsupported PASS).
Each is recorded in place with its correction; the reasoning trail, including a retracted
"top-1 is mechanism-forbidden" reframe, is preserved rather than cleaned away. The net effect: the
*levels* deflated repeatedly; the *mechanism* (the matched-item contrast, which is immune to these
sampling issues) never did.

Other honest limits: results are at TinyModel/SmolLM2 scale; the description baseline (CoTools-style)
is built but not yet run head-to-head; the exponent is a two-decade extrapolation to 10⁶; and
per-cell recall carries a small-N training-amount confound.

## 5. What's next

**Hypothesis (a) is now largely answered** by the synthetic decade (§3.5): the geometry holds
sublinearly with α ≈ 0.65 [0.53, 0.82], usable to ~10⁵ cells, with a standalone 10⁶ address at the
edge. What remains open, and in priority order:

1. **The learned-routing bottleneck — hypothesis (b), still owed.** The synthetic curve used a
   *fixed* address; it does not test whether a model *retrained* with a softmax over ~10⁴ cell tokens
   still routes. That needs the retrain-with-10⁴-tokens build. If the geometry holds but the learned
   vocabulary breaks, that is the measured reason to move to **spec-emission (CN-6** — emit examples,
   not intent, which the execution tier resolves), rather than an ambiguous guess. CN-6 is gated here.
2. **Behaviour vs. language, head to head.** The strong description baseline (`W_d(bge-small(doc))`)
   is built but unrun. The sharp version is the **synthesized-cell** case: an undocumented cell is
   structurally invisible to description-routing but still has a behavioural address — the one
   comparison no description method can match by construction.
3. **The two-tier pipeline at scale.** Beyond the standalone-address envelope, the runtime executes
   the model's top-k to resolve (and *verifies* — a wrong pick is detected, a total miss is a synthesis
   work-order). Measuring end-to-end recovery under the K_exec budget is what turns "confirmed
   mechanism" into "working system."

## 6. Bottom line

Behaviour-derived embeddings give a language model a **computed, zero-shot address** for tools it has
never invoked — confirmed by a double dissociation, a forecast that held across seeds and bases, and
near-zero seed-variance, and situated in a genuine gap between the tool-learning and program-embedding
literatures. Its scaling is now **measured, not assumed**: the address grows sublinearly (α ≈ 0.65,
confirmed by two independent methods across two decades of library size), usable to **hundreds of
thousands of cells on GPU**, with a standalone million-cell address at the edge of the budget and the
tail routed to the two-tier design. The result survived an unusually adversarial internal review with
its mechanism intact and its numbers corrected six times over — which is the strongest thing that can
be said about a first result: it is real, its envelope is measured rather than assumed, and it knows
exactly what it does not yet know (learned routing at scale, and behaviour-vs-language head to head).
