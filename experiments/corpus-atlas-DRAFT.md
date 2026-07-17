# Corpus Atlas — DRAFT instrument spec (chuk-atlas, working name)

2026-07-17. Status: **DRAFT — infrastructure spec, not a preregistration.** No
hypotheses or predictions are pinned here; the DIV-0 experimental prereg is a
separate document and pins there. This doc exists so the instrument every DIV
arm, the CN-10 probes, and future eval reporting will touch is specified and
citable before any code is written.

Feeds: the R1.1 diversity spec (Chris's doc, v0.2 — amendments trail in
`cell-native-architectures/cn7_marshalling_note.md`), the CN-10 TO-PIN
predictions (distance-conditioned predictions are sharper than binary ones),
and every future eval that wants "P-m as a function of measured
training-manifold distance."

Placement rule: **in-repo scripts + SQLite until it survives DIV-0
calibration.** Extraction to a standalone public package (`chuk-atlas`) is
explicitly out of scope for v1 — premature packaging in a multi-session
checkout breeds duplicate scaffolds. CPU-only throughout; must not touch MPS
while the cn8 chain owns it.

## The design in one paragraph

A two-level index over the v11 pretrain — level 1 *surface* (suffix
automaton over the raw token ids: exact-seen and longest-match-per-position in
microseconds), level 2 *skeleton* (the same index over a normalized copy:
digits→`D`, proper names→`N`, plus a POS/dep rendering) — with a third
*model-side* distance on top (base-NLL profile + kNN against sampled mid-layer
residuals). Every probe text gets a **pair** of distances, and the pair is the
diagnostic axis: surface-novel but skeleton-familiar = fresh instance of a
known frame (should marshal); both novel = new frame (collapse-basin risk).
The same index is five tools: (1) manifold-distance meter, (2) diversity
auditor (per-frame cardinality *is* index size, so the R1.1 audit gates become
index queries), (3) marginal-entropy purchaser (dedupe-before-admission),
(4) the train/eval wall enforced mechanically, (5) the DIV-0 retrodiction
instrument. Every distance claim ships with receipts — the nearest training
rows themselves, not just a number.

## 0. Gate A0 — materialize the training stream (blocks everything)

**The 24M pretrain tokens do not exist as a file.** `train_v11_tinystories.py`
(chris-experiments/compilation/15_v11_model) **streams**
`roneneldan/TinyStories` from HuggingFace: phase 1 = 16,000,000 tokens at
seed 42, phase 3 (frozen-FFN attention retrain) = 8,000,000 tokens at
seed 43, MAX_SEQ 256. The `corpus/` directory in that tree is 128 KB of
knowledge-vocab seed text, not the pretrain. Step zero is therefore:

- Re-run the data pipeline **data-side only** (no model), dumping the token
  ids actually consumed, per phase, to disk. Record sha256 per phase file.
- **Determinism audit**: run the dump twice; hashes must match. Pin the HF
  dataset revision and the `datasets` library version in the dump manifest.
  HF streaming-shuffle determinism across library versions is not guaranteed.
- Phase-1 and phase-3 streams stay **separate shards**; seen-counts are
  reported per phase (phase 3 re-exposes text under a different training
  regime — a "seen count" that merges them is a different quantity).
- **If the stream is not bit-reproducible**: the index is over an
  approximation, every downstream distance claim must disclose this, and the
  gate decision (proceed-with-disclosure vs. stop) is recorded here before
  any index is built.

This is the difference between "we can enumerate our pretraining" being a
real capability and a claim with a soft foundation.

## 1. Gate A1 — tokenizer identity is structural, not disciplinary

CN-1 was burned by exactly this (~30% spurious trained-row hits from encoding
through the tiny-model tokenizer artifacts instead of the v11 SP model — see
memory trail and cn1 findings). Therefore:

- The index is built in the **v11 SP id space** (8,599 ids,
  `15_v11_model/v11_tokenizer/v11.model`); the tokenizer file's sha256 is
  baked into the index fingerprint, alongside `v11_train_mask.pt` as an
  ancillary identity artifact.
- The scorer **refuses to return a distance** unless the probe was encoded
  through a tokenizer whose hash matches the index fingerprint — the same
  assert-and-hash move the CN-10 feasibility gate demonstrated.

## 2. Level 1 — surface index

Suffix automaton (or suffix array — at 24M tokens over an 8,599-symbol
alphabet either is a few hundred MB and minutes to build) over each phase
shard plus the merged stream. Queries: exact-substring seen?, count,
longest-match-at-every-position (the per-token novelty profile — the
infini-gram trick at lunch-break scale). Every hit resolves back to stream
positions and decodes to the surrounding training text: **receipts**.

## 3. Level 2 — skeleton index

The identical index over a normalized copy. Normalization happens in **text
space** then re-encodes through the pinned tokenizer:

- digits → `D`, proper names → `N` (spaCy NER + PROPN over the decoded
  stream; children's English, minutes of compute);
- a POS/dependency-tagged rendering as a parallel shard.
- **Alignment requirement (in v1, not discovered mid-build)**: per-token
  novelty profiles live in SP-piece space, normalization decisions live in
  spaCy-token space — maintain char-offset alignment between the two
  tokenizations.

**The skeleton equivalence relation is a versioned choice, not a
definition.** skeleton-v1 = (digits, proper names). Too coarse and the wall
over-rejects; too fine and contamination walks through. Every wall claim
states the skeleton version it is made at.

## 4. Level 3 — model-side distance

Base-NLL per-token profile under v11, plus kNN against a sampled index of
v11 mid-layer residuals (larql-vindex machinery pointed at a much smaller
model). The residual sampling recipe (layers, positions, sample size, seed)
is recorded in the index manifest.

## 5. Scorer output contract

The scorer returns, for any probe: the (surface, skeleton) distance pair,
the model-side distance, and the **nearest training rows themselves** with
stream positions — never a bare number. Every distance claim in any
downstream doc ships with its receipts; this is the greps, automated.

## 6. The catalogue

One SQLite table (JSONL export for diffing), one row per surface pattern:

| column | content |
|---|---|
| skeleton | normalized form, skeleton-version-tagged |
| template | typed slots (COUNT, ENTITY, ITEM, trigger phrase) |
| frame_type | declarative / interrogative / imperative (R1.1 amendment 1) |
| register_band | base-NLL band under v11 |
| counts | per-phase seen-counts from the surface index |
| provenance | harvested-from-position-N / compiler-seed / llm-batch-id / hand |

Deliberately absent from v1: a "measured entropy contribution" column — it is
not mechanically defined as stated in the design conversation; it enters the
schema only with a precise, computable definition.

**Harvest first**: parse the materialized stream once, extract every
sentence's skeleton with typed slots, dedupe by skeleton. What falls out is
the base compiler's actual native inventory, counted, with natural
frequencies — simultaneously DIV-0's enumeration, the S-E generator's raw
material, and the terminals for the S-B grammar compiler (which recombines
*harvested* constituents, not hand-written rules).

**Admission rule (all sources — compiler output, LLM paraphrase, hand)**:
purchase entropy at the margin. Every candidate surface is deduped against
the skeleton index before admission; only novel skeletons are paid for. LLM
paraphrasing under this rule is "reject until novel," which is where its
cost-per-bit gets competitive. Minimal pairs generate mechanically from
catalogue rows (perturb one slot / one trigger / one frame_type).

**Frequency-null rule** (the EX-2 lesson): harvested natural frequencies are
inventory. Any *claim* of over/under-representation requires its null —
never read a raw frequency as signal without the baseline the generating
mechanism implies.

## 7. The train/eval wall

FS-bank items are admitted only if skeleton-disjoint from the catalogue —
the eval-contamination guarantee becomes a check, not a discipline. Two
requirements the conversation-side design missed:

- The wall-checker reports matches at **both** levels (surface and
  skeleton), so near-misses are visible rather than silently passed.
- The wall indexes **all training exposure, not just pretrain**: the CN-7
  midtrain corpora (S1/S2) and the CN-8 corpora (B / A-ex / A-tok) enter as
  separately-fingerprinted shards. Without this, "disjoint from all
  training" is quietly false for every post-midtrain model.

## 8. DIV-0 calibration protocol (the forking-paths fix)

The three distances must retrodict the known ordering of the existing
battery, anchored in CN-7 evidence (`cn7_marshalling_note.md`):

    trained row < fresh operands (in-range divisor sweep)
                < variant phrasing (marbles/friends)
                < interrogative register (S3 collapse)

**Selection/confirmation split, pinned here because "whichever retrodicts
becomes the gating measure" is circular as stated:**

1. The existing battery is the **dev set**. Gating-measure selection happens
   there and only there.
2. A **held-out prompt set is frozen — committed and hashed — before any
   scoring runs.** The selected measure must reproduce the ordering on it.
3. Selection may not be revisited after the held-out set is seen. If the
   held-out confirmation fails, that is a reported result about the
   instrument, not a license to reselect.

## 9. Build order (each gate blocks the next)

1. **A0** stream dump + determinism audit (half a day, mostly compute)
2. **A1** tokenizer pin + index fingerprint scheme
3. Surface index + scorer with receipts (a day)
4. Skeleton pipeline (normalization, alignment, second index) (a day)
5. Harvest + catalogue schema + admission rule (a day or two)
6. Wall-checker over all shards incl. midtrain corpora
7. DIV-0 calibration per §8 — the generators (S-B compiler, S-E, LLM
   paraphrase loop) are plug-ins after this point, not before

## 10. What this is not

Not a preregistration — no predictions, no thresholds. Not a public tool
yet. Not a license to touch CN-10 (predictions stay TO-PIN until the CN-8
readout) or to spend MPS. It is the instrument those things will use, built
on the only substrate where it is cheap: a training set small enough to
enumerate. Reporting generalization as a curve over exact training-manifold
distance is a capability that currently exists nowhere at any scale — but
only if gates A0 and A1 hold, which is why they are gates.
