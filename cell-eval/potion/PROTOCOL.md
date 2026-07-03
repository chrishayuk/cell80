# cell-potion training protocol

Implementation record for `docs/cell-potion-training-spec.md`. Everything here was
fixed **before** the one-shot frozen eval.

## Corpus generation (datasets/potion-train-pairs.jsonl)

- Source of truth: the 100 seed-library manifests (`id`, `summary`, `tags`,
  `signature`) — nothing else. The frozen eval (`datasets/retrieval.jsonl`) was
  never read by any generation or training step.
- Confusable map: for each cell, top-4 nearest neighbours by cosine over the
  harness manifest-doc text (`_doc`: id + summary + tags) under
  `ollama:nomic-embed-text`. Training input only; the eval judge is unchanged.
- Authoring: 5 parallel LLM agents, 20 cells each, manifests + confusables passed
  **inline** (agents run with no file access — eval contamination is structurally
  impossible). Per cell: 8 paraphrase queries (synonym-rotated, register-varied)
  + 4 adversarial near-misses, each skirting one named confusable whose id is
  recorded first in `hard_negatives`.
- Mechanical direct anchors: +1 row per cell, query = the manifest summary itself.
- Validation: 100/100 cells at exactly 8+4, all cell/negative ids resolve,
  zero duplicate queries.

## Decontamination audit (audit_overlap.py → overlap-audit.json)

Pre-registered rule: drop any training row whose max cosine to any frozen eval
query is >= 0.92 under the neutral embedder (`ollama:nomic-embed-text`), or that
is a case-insensitive exact duplicate. Eval text flows only *out* (row removal),
never into generation or training.

Result: 23/1300 dropped (1.8%), max NN-sim 1.0 (one training query was
effectively identical to an eval query — the leak this audit exists to catch),
mean 0.719, p95 0.875. Clean corpus: 1277 rows.

## Training (train.py)

- Architecture: the potion-retrieval-32M token table (63091x512 f32), trained
  directly; pooling = plain mean + L2 normalise, verified bit-exact against
  `StaticModel.encode` before training, so the artifact behaves in the harness
  `Embedder` exactly as in training. No transformer teacher anywhere.
- Loss: InfoNCE (CE over ALL 100 manifest docs as the candidate set — the full
  retrieval space, strictly stronger than in-batch negatives at this scale)
  + lambda-weighted restricted CE over {positive} ∪ authored hard negatives.
- Optimiser: Adam over the table, batch 256, seed 80, deterministic.
- LR band extended per the HF static-embeddings recipe (lookup tables tolerate
  ~100x transformer LRs; they train at 0.2 SGD from random init — we warm-start
  with Adam, so the aggressive band sits lower).

## Dev-split discipline (the eval-shot budget)

- Dev = deterministic ~25% hash split of the **generated** corpus
  (`sha256(query) % 4 == 0`). Hyperparameters (tau, lambda, lr, epoch via
  best-epoch selection) chosen ONLY on dev (`--sweep`, `sweep-results.jsonl`).
- The frozen eval is touched **once**, after the artifact is final:
  `cell-eval tiers --embed-model <artifact>` + per-model theta calibration at
  the 0.75 adversarial precision floor. No iteration against the frozen set;
  whatever that single run says is what gets banked.

## Latency gate (pre-registered measurement)

"In-process embed <= 100 us/query" = warm single-query `Embedder.encode`
(tokenize + gather + mean + normalise), median over 2000 calls, in the harness
Python path. Baseline potion-retrieval-32M measures 32 us median / 45 us p99
(the banked 1.7 ms/query bake-off figure includes eval-loop overhead, not the
in-process floor). cell-potion shares tokenizer and table shape, so its latency
is identical by construction — but it is measured anyway, not asserted.

## Regeneration (added after the banked run)

The banked corpus above was authored by session agents with manifests passed
inline. The committed, mechanical regeneration path for library growth is
`cell-eval potion-pairs` (`src/cell_eval/potion.py` + offline tests): same row
shape and per-cell counts (8 paraphrase / 4 adversarial skirting a named
confusable, recorded first in `hard_negatives` / 1 direct anchor), same
neutral-embedder confusable map, manifests-only authoring, and the same
validation checks — then this audit (`audit_overlap.py`) and `train.py`,
unchanged. `--cells new_a,new_b` regenerates rows for new cells only, per the
spec's growth invariant (never touch the eval rows). The banked numbers were
produced from the committed corpus, not from this CLI.
