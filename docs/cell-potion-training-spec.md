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
