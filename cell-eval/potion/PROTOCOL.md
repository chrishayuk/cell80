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

## v2: margin-shaped training (pre-registered 2026-07-04, BEFORE any v2 eval)

Motivation (from the banked v1 numbers, no new eval reads): on adversarial,
cell-potion v1 and nomic have IDENTICAL ungated P@1 (0.538), but nomic answers
0.46 at the floor vs v1's 0.192 — the entire coverage gap on that split is
margin geometry, not ranking. Paraphrase shows the same shape (0.604 ungated
vs 0.396 answered). v1's InfoNCE never asked for separation; v2 adds a hinge
on the raw-cosine margin: mu * max(0, gamma - (s_pos - max_{c != pos} s_c)),
alongside the unchanged InfoNCE (mu = 0 reproduces v1 exactly; the banked
gradient checks pin that path).

Dev selection (generated-corpus dev split ONLY, as v1): sweep mu in {0.5, 1, 2}
x gamma in {0.1, 0.2, 0.3} at v1's winning tau = 0.05 / lr = 0.05, 30 epochs,
best-epoch + config chosen by the GATE-PROXY score: calibrate theta_dev on dev
(smallest theta with dev-adversarial precision-on-answered >= 0.75, pure-cosine
margins over all 100 docs), score = summed dev answer rates at theta_dev.
Tie -> higher dev adversarial answer rate.

The v2 kill gate (one shot at the frozen set, decided before running):
- answered-coverage STRICTLY beats v1 on every split (v1: direct 0.814,
  paraphrase 0.396, adversarial 0.192) at v2's own calibrated theta
  (0.75 adversarial precision floor, same judge, same blend);
- the adversarial win must be NON-THIN: >= +2 queries (>= 7/26 = 0.269) —
  v1's one-query margin must not be repeatable as noise;
- latency stays static-class: <= 100 us warm single-query in-process
  Embedder.encode median (measured, not asserted);
- anything else — including strict-but-thin adversarial, or two splits of
  three — is a KILL, banked in the spec Result either way.

Consequences, fixed now: earned-in -> the v2 table replaces the `cell-potion`
alias artifact (potion/model) and its theta in OPERATING_POINTS; the v1 row is
preserved in the spec's Result history. Killed -> v1 stays the banked artifact,
v2 numbers recorded as the measured ceiling of margin shaping. This is the
final static-tier training experiment either way; the remaining adversarial
residue belongs to tier 3.

Eval-shot ledger for the frozen set: v1 run (2026-07-03) = 1 read; v2 run = 2nd
read. Any experiment beyond v2 requires an independently authored eval
extension FIRST.

### v2 amendment (2026-07-04, before any frozen-eval read)

The pre-registered dev selection criterion (theta_dev calibrated at dev-adversarial
precision >= 0.75) was DEGENERATE as an instrument: the authored near-misses are
harder than eval adversarial queries and pure cosine has no tier-1 blend, so
theta_dev saturated at ~0.30 with zero dev-adversarial coverage for every config —
all 9 swept configs tied at 0.331 and best-epoch selection collapsed to epoch 2
(~warm start). First sweep banked as sweep2-results.jsonl.

Amended dev criterion (dev-side only; the frozen set remains at 1 read, the v2
shot still unspent): net coverage at a FIXED cosine margin M0 = 0.15 — per split
P(correct AND margin >= M0) - P(wrong AND margin >= M0), summed. M0 comes from the
harness scale (blended theta ~= 0.75 x cosine margin; operating band 0.11-0.14 ->
~0.15-0.19 cosine), fixed across configs, never tuned per run. Gamma grid shifts
to {0.2, 0.3, 0.5} (hinge stops pushing at gamma, so gamma must exceed M0; the
original 0.1 could not reach the operating band by construction). The v2 KILL GATE
IS UNCHANGED — only the dev-side selection instrument moved.
