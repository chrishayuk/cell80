# cell-potion — a domain-**trained** static embedder (kill-gated experiment spec)

*Status: specified, not started. Runs after manifest enrichment / alongside tier 3 —
neither blocks on it. One day-scale experiment with a pre-registered kill gate, in the
`synth_value_gate` discipline: the gate decides, not the demo.*

*Design decision: this is a **training**, not a distillation. No transformer teacher —
the static token vectors are trained directly on domain pairs. Distilling would cap the
model at a generic teacher's view of a domain the teacher never saw; training on the
domain's own pairs lets the tiny model spend its entire capacity on the one
distribution it will ever serve. It also keeps the experiment honest as a measurement
of "what can static vectors learn *from this domain*", not "how much of a transformer
survives compression".*

## Why this exists (and why nothing heavier does)

The escalation ladder's rung 2 is specced **µs–ms**, and the SOMA organ layer runs
reflexes at kHz–MHz. No served transformer sits in that tier: the bake-off
(`cell-eval/baselines/embed-bakeoff.json`) puts the best supported models at
31–119 ms/query over HTTP — three to four orders of magnitude off the reflex budget.
The only architecture that fits is a **static embedder** (model2vec-class: one
vector per vocabulary token; a query embed is a table gather + mean — tens of µs
in-process, no server). The problem is coverage: generic potion-32M answers only
0.72 / 0.23 / 0.15 (direct / paraphrase / adversarial) at the calibrated gate, versus
nomic's 0.91 / 0.47 / 0.46.

So the question this experiment answers is **not** "can we beat nomic" — it is:

> Can a static model **trained on this domain** claw back answered-coverage while
> keeping static-model latency?

The domain is the best case for static vectors: a small vocabulary (math/tool words),
short queries, short documents. If domain training can't win here, it can't win
anywhere — which is also worth knowing, and banking.

## What gets built

1. **A synthetic pair corpus, generated from manifests only.** For every cell in the
   seed library, an LLM authors N paraphrase queries + M adversarial near-miss queries
   (nearest-neighbour cells named as the confusable target), from the manifest text
   alone. **The frozen eval set (`datasets/retrieval.jsonl`) is never read by any
   training step** — it is the held-out judge, untouched.
2. **A trained static model.** Architecture: a token-vector table + (weighted) mean
   pooling — the model2vec inference shape, so the artifact drops into the existing
   `Embedder` unchanged. Training: contrastive (InfoNCE-style) over the pooled
   vectors on (query ↔ manifest-doc) positives with in-batch negatives — and the
   authored near-miss queries as **hard negatives**, which is exactly the split the
   gate cares about. Tokenizer: a small domain vocabulary (or a stock tokenizer —
   implementation's choice). Initialization: random, or warm-started from a generic
   static table — **not** from a transformer teacher; the gradient signal is the
   domain pairs, nothing else.
3. **A bake-off row.** The artifact runs through the existing harness unchanged:
   `cell-eval tiers --embed-model <path>` → blended rerank → margin gate → per-model
   θ calibration at the 0.75 adversarial floor.

## The kill gate (pre-registered)

cell-potion **earns in** iff, on the frozen eval set, at its own calibrated θ:

- **answered-coverage strictly beats potion-retrieval-32M on every split**
  (baseline to beat: 0.72 / 0.23 / 0.15), with precision-on-answered still clearing
  the same floors; **and**
- **latency stays in the static class**: in-process embed ≤ 100 µs/query (no server,
  no network — measured, not asserted).

Anything else — including "close on two splits" — is a kill. A kill is banked in
`baselines/` and this spec gets a *Result* section either way; a negative here is the
measurement that static-plus-domain-training has a ceiling, which prices rung 2's
floor honestly.

## Protocol invariants (the part that keeps the number honest)

- **Train/eval separation is structural, not procedural**: the training corpus is
  generated from manifests; the eval set was authored independently and stays frozen.
  If the library grows before the run, regenerate training pairs for new cells but
  do not touch the eval rows.
- **Same judge for every candidate**: the existing gate + floor + blend (α = 0.25).
  No bespoke metric for the new model.
- **θ is calibrated per model** (`OPERATING_POINTS`), as for every bake-off entry —
  margin scale depends on embedding geometry.
- **Report all three splits.** A blended P@1 hides exactly the failure the gate
  exists to catch.

## Boundaries (already banked — do not re-run)

- **No transformer teacher, no distillation.** See the design decision above.
- **No custom transformer embedder.** Same latency class as nomic, real training
  cost, and an overfitting trap on a 181-query eval. The supported models own that
  tier.
- **No learned text→cell selector.** Three measured losses to retrieval
  (escalation-ladder do-not-build). Trained *vectors for retrieval* are in scope;
  a model that *picks cells* is not.
- **Known ceiling, stated up front:** the residual paraphrase misses are same-shape
  siblings (`min`/`max`, `gcd`/`lcm`) invisible to any text geometry. Those belong to
  tier 3 (behaviour). This experiment buys coverage on the text-separable residue
  only — that is the honest size of the prize.

## Deliverables

- The model artifact (path recorded here on completion) + the pair-generation script
  and corpus under `cell-eval/` (committed: the corpus is what makes the run
  repeatable).
- A `baselines/embed-bakeoff.json` row + calibration entry.
- The *Result* section of this spec: earned-in or killed, with the numbers.

## Result (2026-07-03): EARNED IN

One-shot frozen eval (θ = 0.11, calibrated at the 0.75 adversarial
precision-on-answered floor; full curve in `cell-eval/potion/frozen-eval-result.json`):

| split | answered-coverage @ precision | potion-32M baseline | gate |
|---|---|---|---|
| direct | **0.814** @ 1.00 | 0.72 @ 1.00 | beats |
| paraphrase | **0.396** @ 0.81 | 0.23 @ 0.83 | beats |
| adversarial | **0.192** @ 0.80 | 0.15 @ 0.75 | beats (thin: 5/26 vs 4/26 — one query) |

Latency: **34 µs median / 48 µs p99**, warm single-query `Embedder.encode`
in-process — the pre-registered measurement (the old banked 1.7 ms/query for
potion-32M was eval-loop overhead; base potion measures 32 µs the same way).
Both gate halves pass: coverage strictly beats potion-retrieval-32M on every
split with the adversarial precision floor held, and latency stays in the
static class.

**Ungated P@1 tells the sharper story**: paraphrase 0.604 — above nomic's
0.566. The trained static vectors *rank* this domain better than the served
transformer; the remaining coverage gap to nomic (0.396 vs 0.47 answered) is
margin geometry at the gate, not ranking quality. Adversarial ungated 0.538
exactly ties nomic. Rung 2's floor is now a domain-trained 34 µs model, not a
generic one.

Honest caveats, banked with the number:
- The adversarial coverage win is one query at n = 26. Real, pre-registered,
  not tuned — but thin. Nomic still owns adversarial coverage (0.46).
- The corpus decontamination audit dropped 23/1300 training rows at the
  pre-registered 0.92 nomic-similarity threshold, including one row identical
  to a frozen eval query (max sim 1.0) — without the audit this result would
  carry an asterisk (`cell-eval/potion/overlap-audit.json`).
- The dev sweep showed the authored-hard-negative loss term earning λ = 0: with
  all 100 docs in the softmax every confusable is already a negative each step.
  The near-miss *queries* still matter (they are training rows); the extra loss
  term did not.
- The artifact is library-version-bound (a domain lexicon, zero transfer to
  future cells). Retraining cost: corpus regeneration for new cells (LLM
  authoring, ~5 min wall-clock for 100 cells) + `potion/train.py` (~2 min CPU,
  deterministic seed 80). That is the maintenance price of rung 2.

Deliverables: protocol + scripts in `cell-eval/potion/` (PROTOCOL.md, train.py,
audit_overlap.py, overlap-audit.json, sweep-results.jsonl,
frozen-eval-result.json), corpus `cell-eval/datasets/potion-train-pairs.jsonl`
(+ `.clean.jsonl`), bake-off row `cell-eval/baselines/embed-bakeoff.json`,
harness alias `--embed-model cell-potion` (θ in `OPERATING_POINTS`). The
model artifact lives at `cell-eval/potion/model/` (129 MB, not committed;
rebuild is deterministic from the committed corpus). The pair-generation
deliverable is committed post-run as `cell-eval potion-pairs` (same protocol
and row shape; see PROTOCOL.md §Regeneration) — the banked corpus itself was
agent-authored and is unchanged.

Winning config (selected on the generated-corpus dev split only, pre-registered
criterion = sum of split accuracies): τ = 0.05, λ = 0.0, lr = 0.05 (Adam),
best epoch 6 — the most aggressive lr in the grid, consistent with the HF
static-embeddings finding that lookup tables want ~100× transformer LRs.

## Result v2 (2026-07-04): margin-shaped training — EARNED IN

v1's banked anomaly (identical ungated adversarial P@1 to nomic, 2.4x less
certified coverage) said the gap was margin geometry, not ranking. v2 added one
term to the objective — a hinge mu * max(0, gamma - (s_pos - max_other)) on the
raw cosine margin (mu = 2, gamma = 0.5, selected on the generated-corpus dev
split only) — and took the second, pre-registered shot at the frozen set:

| split | v2 @ theta = 0.14 | v1 | nomic |
|---|---|---|---|
| direct | **0.824** @ 1.00 | 0.814 @ 1.00 | 0.91 @ 0.99 |
| paraphrase | **0.472** @ 0.96 | 0.396 @ 0.81 | 0.47 @ 0.80 |
| adversarial | **0.308** (8/26) @ 0.75 | 0.192 (5/26) @ 0.80 | 0.46 @ 0.75 |

All three gate conditions pass: strict improvement on every split, adversarial
+3 queries (the >= +2 non-thin bar held), latency 34.5 us median / 49 us p99.

**The headline: paraphrase answered-coverage now MATCHES nomic (0.472 vs 0.47)
at higher precision (0.96 vs 0.80) and ~1000x lower latency.** On the
text-separable residue, the domain-trained static tier no longer concedes
anything to the served transformer. Adversarial closed from 0.192 to 0.308
against nomic's 0.46 — margins bought back half the remaining gap; what's left
is the same-shape-sibling residue that belongs to tier 3.

Protocol notes banked with the number:
- The pre-registered dev selection instrument (precision-calibrated theta_dev)
  was degenerate — all 9 configs tied, best-epoch collapsed to the warm start.
  Amended BEFORE any frozen read to fixed-threshold net coverage (M0 = 0.15);
  the degenerate sweep is banked as sweep2-results.degenerate-criterion.jsonl.
  The kill gate itself never moved.
- Dev adversarial net coverage stayed NEGATIVE for every config (confidently
  wrong 3-4x more often than confidently right on authored near-misses) — yet
  the frozen adversarial split improved 5/26 -> 8/26. The authored near-misses
  are harder than the eval's; dev pessimism on this split is structural.
- Eval-shot ledger: the frozen set has now been read twice (v1, v2). Any
  further experiment requires an independently authored eval extension FIRST.
  This closes the static tier's training program.

The v2 table replaced the `cell-potion` alias artifact (potion/model,
rebuild: `train.py --mu 2 --gamma 0.5`, seed 80, deterministic) and theta in
OPERATING_POINTS (0.11 -> 0.14); the bake-off row carries v2.
