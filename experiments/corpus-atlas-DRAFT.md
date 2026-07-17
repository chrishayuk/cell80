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

**GATE VERDICT (2026-07-17): A0 CLOSED — PASS.** Artifacts in
`corpus-atlas/` (`a0_dump_stream.py`, manifests; the ~96 MB `.u32` dumps
stay untracked):

- Phase 1: 16,000,000 tokens exactly (62,500 chunks, 60,191 stories),
  sha256 `883b483c…bebbfb`. Phase 3: 8,000,000 exactly (31,250 chunks,
  30,452 stories), sha256 `bd0dd555…adb3efd`. uint32-le, BOS id 2.
- **Determinism**: two independent runs, per-phase sha256 identical;
  manifests identical modulo run tag. Env pinned: Python 3.12.2,
  `datasets` 3.2.0, `sentencepiece` 0.2.0, HF revision `f54c09fd…`,
  dataset last modified 2024-08-12 — predating the v11 training run
  (2026-04-11), so content drift since training is ruled out.
- **Training-era identity** (`a0_mask_check.py`): replicating
  `build_train_mask`'s construction (first 5,000 unshuffled stories +
  SP specials + capitals list) reproduces `v11_train_mask.pt`
  **bit-for-bit** (8,599 = 8,599, zero differing ids) — dataset order and
  tokenizer id mapping verified against an artifact from the training
  era, which run-to-run self-consistency alone cannot show.
- **Census finding**: the stream exercises **17,158 distinct ids**
  (phase 1: 15,758; phase 3: 13,207); the mask's 8,599 is a decode
  *whitelist* (first-5k-stories vocabulary + capitals), not a trained-ids
  census. Ids outside the mask are rare tails: median count 3, max 267,
  0.22% of the stream. Any claim keyed to "trained ids" must say which of
  the two sets it means.
- Residual (disclosed, not blocking): the `datasets` version at training
  time is unrecorded; 3.2.0's streaming order is proven stable and
  mask-consistent with the training era, which bounds the risk to a
  library-behavior change between that era's version and 3.2.0 that
  somehow preserves the first-5k prefix order. No stronger check exists
  without a training-time stream hash.

### 0.1 Mask-audit and first cross-instrument check (2026-07-17)

**Whitelist-as-census audit** (who keyed what to 8,599): `cn7_0_yield.py` /
`cn7_deck.py` use the mask as a decode whitelist — its designed purpose;
effect is only that ~8.6k rare trained ids were excluded from decode
(immaterial to the R1 verdicts; cn7_deck unions corpus ids anyway).
`cn10_readout.py` computes ranks over the full unmasked vocab — clean. The
one census-keyed *measurement* was CN-1 §8.1's "29.8% trained-row hits
(chance 12.1%)": recomputed against the true census, 32.0% vs 24.1%
chance — the wrong mapping is *near-chance* on trained rows, and the
2.5×-chance flavor of the old framing was a whitelist artifact. The CN-1
verdict (mostly-untrained substrate: 68% of context tokens on never-trained
rows) is unchanged. Nothing graded flips.

**Digit-prior retrodiction** (`retro_digit_prior.py` — the atlas
retrodicting CN-10's smoke, two instruments on one prior): CN-10's median
answer-digit ranks vs corpus counts in the materialized stream. The
pre-specified naive check (global unigram) gives Spearman −0.37 — weak,
right sign. Receipts on the misfits exposed the conditioning error: the
smoke's measured quantity is the *first* digit piece of the answer, a
number-initial position, while unigram counts mix roles ('0' lives
number-finally in 10/20/30 — just 3 initial occurrences in 24M tokens,
rank 663; '1' owns the counting register 10/12/100). Conditioning on
number-initial occurrences: **Spearman −0.77** (−0.87 excluding '3',
whose 1,992 hits are the frame-bound "a 3 year old" idiom that evidently
does not transfer to the post-`=` slot). Read: corpus enumeration and
behavioral readout agree on what the digit prior *is* — the two-routes
rule crossing experiment boundaries — and the '3' residual is a
receipt-documented example of frame-sensitivity: the prior is not a
unigram table. Caveat: counts are tens-to-hundreds; ordering is coarse.

**Design ruling for CN-10, settled by the '3' outlier:** the behavioral
prior (shuffled-rank control) stays the *operative* delta-from-prior
correction; the corpus count is its validating second route, not its
replacement. The model's prior at the post-`=` position is conditioned on
more context than any position-binned corpus count captures —
delta-from-corpus-prior would import exactly the residual the idiom
exposes. Corpus count as receipt, shuffled rank as instrument.

**M3 dissociation (for cross-reference when DIV-0 findings are committed
in-repo):** DIV-0's M3 read "What is 25 multiplied by 32?" as *more*
familiar to the base than its lexical variant. The surface index shows why
that reading was true and useless at once: coverage of the collapse-basin
question is a 4-token opener (`What is `, count 4, story dialogue) —
register-warm, coverage-dead after four tokens. Base-NLL integrates over
the whole string and is fooled by the warm opener; longest-match-per-
position sees the cliff exactly where it is. The atlas dissociates what M3
conflated — the mechanical justification for the M3-retrodicts-nothing
verdict.

## 1. Gate A1 — tokenizer identity is structural, not disciplinary

CN-1 was burned by exactly this (~30% spurious trained-row hits from encoding
through the tiny-model tokenizer artifacts instead of the v11 SP model — see
memory trail and cn1 findings). Therefore:

- The index is built in the **v11 SP id space**: the full 71,261-piece
  `15_v11_model/v11_tokenizer/v11.model`, of which exactly **8,599 ids are
  train-exercised** (`v11_train_mask.pt` — "8,599 ids" in earlier notes was
  this subset, not the id space; the A1 assert caught the conflation on its
  first run). The tokenizer file's sha256 is baked into the index
  fingerprint, with the train mask as an ancillary identity artifact; each
  phase dump records its distinct-id census for the mask cross-check.
  Practical corollary: ids reach 71,260, so dumps are uint32, not uint16.
- The scorer **refuses to return a distance** unless the probe was encoded
  through a tokenizer whose hash matches the index fingerprint — the same
  assert-and-hash move the CN-10 feasibility gate demonstrated.

## 2. Level 1 — surface index

Suffix **array** (numpy prefix-doubling), one per phase shard — a pure-
Python suffix automaton at 24M symbols is a memory blowup for no query
benefit. Queries: exact-substring seen?, count, longest-match-at-every-
position (the per-token novelty profile — the infini-gram trick at
lunch-break scale). Every hit resolves back to stream positions and
decodes to the surrounding training text: **receipts**.

Two semantics decisions, pinned: (a) **"seen" means within a 256-token
training chunk** — the model never attended across chunk boundaries, so a
sentinel between chunks makes straddling matches impossible; text flowing
across a boundary in the source story was never seen as a sequence.
(b) **Verbatim-training probes must be queried by raw token ids** — SP
decode→encode is not identity for slices starting mid-sequence, so
text-entry profiles undercount at mid-string starts (caveat, not defect).

**BUILT (2026-07-17, `atlas_surface.py`)**: 16,062,500 + 8,031,250 symbols
(with sentinels), ~3 min CPU, arrays untracked (~96 MB), fingerprint in
`surface_index_meta.json` (tokenizer sha256 + stream sha256s; query path
refuses a hash-mismatched tokenizer per §1). Smoke: verbatim in-chunk
32/32 tokens count 1; boundary straddle capped at 16/32; novel
interrogative probe maxes at 4 tokens ('What is ', 4 occurrences, receipts
decode to story dialogue) — while 'Once upon a time, there was a little
girl' matches full-length with **count 8,245**. The diagnostic axis
(trained row ↔ novel frame) is live.

## 3. Level 2 — skeleton index

The identical index over a normalized copy. Normalization happens in **text
space**; the index is built over **word-level vocabulary ids** — a disclosed
amendment to this draft's original "re-encode through the pinned tokenizer":
any textual D/N marker either collides with real corpus text or encodes to
unk through the SP model (collapsing the D/N distinction), and word-level
ids are the natural space for frames ("N gave N D apples" = 5 symbols) and
for the catalogue harvest.

**skeleton-v1, pinned** (`atlas_skeleton.py`): spaCy en_core_web_sm tokens
(tok2vec+tagger+attribute_ruler+ner; parser excluded); any digit-bearing
token → `D`; PROPN or PERSON/GPE/LOC/ORG/FAC entity → `N` with contiguous
runs collapsed (a multi-word name is one referent); all else lowercased
word → vocab id. Chunk text is reconstructed from ORIGINAL pieces
('▁'→space, control ids zero-width, byte-fallback runs decoded together as
UTF-8) — never decode→re-encode. Probe-time OOV words map to a
never-matching symbol: an unseen word is automatically a novelty frontier.
A POS/dep-tagged rendering as a parallel shard stays open for a later
skeleton version.

**BUILT + ALIGNMENT GATE PASSED (2026-07-17)**: 12,297,160 + 6,154,160
skeleton symbols, vocab 18,456 words; ~35 min in the pinned venv
(`skeleton-requirements.txt` — system spaCy is binary-broken and the
system python is owned by live training chains). Smoke: verbatim slice
20/20 count 1 with receipt resolving to the source chunk; sentinel
straddle capped 8/16; **alignment round-trip 100.0% of 500 random
positions**; two-distance demo — "One day, a little girl named Zorblax
found 7 shiny pebbles." scores surface max 6 vs skeleton max 9
(`one day , a little girl named N found`, count 43, receipt: "One day, a
little girl named Sue found Tim hiding."). The two-distance axis
(surface-novel / skeleton-familiar = fresh instance of a known frame) is
live with receipts on both routes; `atlas_skeleton.py score --text` is
the spec-§5 scorer.

**Equivalence audit vs DIV-0's metrology (2026-07-17,
`skel_v1_equiv_check.py`): NOT EQUIVALENT — divergence one-sided and
mechanism-confirmed.** Two skeleton definitions coexist: DIV-0's frozen
verdict sits on metrology-M1 (deterministic capitalization-lexicon,
v11-train-plan/div0), the atlas on skeleton-v1 (spaCy). On the frozen
CN-7 corpus (sha-verified), v1 gives S2 = 4–6/frame vs the pinned 2, 31
total vs 12; 37 frames diverge, every one inflated on the v1 side. The
corpus is exonerated (exactly one warm-up template exists textually;
15,008 rows). Confirmed mechanisms, all spaCy noise on template text:
same-surface name-tag instability ("Lily… Lily" → N/lily, reproduced on
demand), entity-span boundaries absorbing adjacent words ("gave", "so"),
and symbol mistags in call grammar (=, >, ⟨cell⟩ → N). Coverage close but
not identical (D-B2 0.8438 vs 0.8214; D-B5 0.3636 vs 0.3077).
**Consequence, pinned:** skeleton-v1 does not inherit DIV-0's authority on
template/call-bearing corpora — metrology-M1 stays the operative
normalizer for DIV-1 audit rows and the midtrain wall-checker; v1's
domain is the natural-prose pretrain, where its frame cardinalities read
as mild upper bounds (it splits, never merges). Open follow-up: an
m1-mode renderer in the atlas so DIV quantities compute atlas-side under
the frozen definition — same instrument, both normalizers, no
incommensurability.
- **Alignment requirement (in v1, not discovered mid-build)**: per-token
  novelty profiles live in SP-piece space, normalization decisions live in
  spaCy-token space — maintain char-offset alignment between the two
  tokenizations. Normalization re-tokenizes, so skeleton-index positions do
  NOT align to surface-index positions for free; the alignment map is where
  silent bugs will live. **Gate: the alignment map gets its own smoke of
  the straddle/verbatim class** (known skeleton slice → surface positions
  and back, boundary cases included) **before any two-distance number is
  believed.**

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

**HARVESTED (2026-07-17, `harvest_catalogue.py` → `harvest_summary.json`;
the 290 MB catalogue jsonl stays untracked).** 1,360,050 distinct v1
frames / 1,308,777 m1 frames over the 24M-token stream — the two
renderers agree on the distribution to three decimals (hapax share
0.9398 vs 0.9396, identical quantiles, head max 9,019 vs 9,034; the
+3.9% v1 frame count is the splits-never-merges signature and does not
move the histogram). Two-routes, passed on the harvest itself.

**The DIV-1 calibration answer:** the pretrain's frame-diversity
distribution is extreme-Zipf — **94% of frames occur exactly once**
(p50 = p90 = 1, p99 = 5); the head is a handful of formulae
("once upon a time , there was a little girl named N ." 9,019× with 185
distinct fillers; dialogue attribution; "the end ."). DIV-1's seeded
levels land as percentiles: cardinality **1 = the pretrain's modal frame
(94.0th)**, 8 = 99.5th, 64 and 512 = 100th; in distinct-filler terms 64
is past the 99.9th percentile and **512 exceeds the pretrain's maximum
(303 v1 / 331 m1)**. So the ladder spans typical-to-super-corpus — a
good design property, but the reading changes: 64 is not "typical
diversity," it is already head-of-distribution, and the pretrain's
fluency is built overwhelmingly on hapax frames plus a tiny formulaic
head with O(100) fillers. Caveat: full-sentence skeletons are the unit
and long sentences are combinatorially near-unique; the sharper
calibration is length-conditioned (frames of ≤ ~8 symbols, comparable to
DIV-1 templates) — a cheap groupby over the catalogue, queued.

**REGISTERED PREDICTION — length-conditioned calibration cut (committed
BEFORE the groupby runs; the computation is a one-liner away and this
prediction is worthless afterward).** The 94%-hapax finding puts pressure
on the diversity law: a fluent compiler trained overwhelmingly on
singleton frames shouldn't exist under a naive reading of "surface
diversity is the signal." The expected resolution: the diversity ledger
lives at SHORT-frame granularity — hapax sentences share
massively-repeated sub-frames, and that is where equivalence-class
evidence accumulates. Prediction for frames of ≤8 skeleton symbols:
hapax share **collapses below 0.5** (vs 0.94 overall), occurrence p50
≥ 2, filler distributions turn heavy (a substantial short-frame body
with distinct-filler counts in the tens-to-hundreds), and DIV-1's levels
8 and 64 land INSIDE the short-frame body rather than at the 99.5th+
percentile. Falsifier branch: short frames also mostly hapax (share
≳ 0.8) → a fluent model trained on singletons at every scale, and the
diversity law has a genuine problem. Disclosure: this prediction is
informed, not blind — the already-computed head frames are visibly short
formulae ("N said ." 7,038×); the open quantity is the hapax share
within the short-frame class, not the existence of a repeated head.

**OUTCOME (same day, `length_conditioned_cut.json`): PREDICTION FAILED —
falsifier branch fired.** ≤8-symbol frames (364,205 of them, 20.2% of
sentence tokens): hapax share **0.8637** (predicted <0.5; falsifier
≳0.8), occurrence p50 = 1 (predicted ≥2), DIV-1 level 8 at the **98.6th**
percentile (predicted inside the body), 97.4% of short frames have
exactly one filler tuple. The 9–16 and >16 bands are worse (hapax 0.963
and 0.987). At sentence granularity the pretrain is singleton-dominated
at every length. What this outcome does NOT test: the sub-sentential
ledger — repeated constituents/n-grams shared ACROSS hapax sentences —
which was named in the resolution hypothesis but is not measured by a
length cut on whole-sentence frames. That is the one remaining live
branch, registered separately below; if it also fails, the diversity law
has a genuine problem at every scale.

**Pinned reading for the DIV-1 spec (to fold into the R1.1-side doc
before the arms exist):** DIV-1's top arm is not "very diverse" on an
abstract scale — at 512 fillers per frame it is **super-corpus**,
exceeding the pretrain's most diverse frame (303 v1 / 331 m1). Ergo: a
knee at ≤64 sits inside the pretrain's natural range and says canonical
competence is buyable at head-of-distribution diversity; a knee only
near 512 says the marshalling compiler needs supervision beyond anything
natural text provided — itself a strong claim about why the wall kept
appearing.

**CN-8 band distances** (`cn8_band_distances.json`, committed before any
grading verdict exists): pretrain-distance is FLAT across B0/B1/B2
(surface max-match mean 2.7 → 2.65; skeleton 1/3 constant) — as designed,
since the bands vary operand range, not register. The informative axis
for the exact-vs-distance curve is distance to the MIDTRAIN corpus, which
makes the §7 wall-checker shards the last missing piece for the P-m
curve.

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

**FROZEN (2026-07-17):** `corpus-atlas/div0_heldout.json`, sha256
`6549a2cf1e653d87cc436f3a30022b769d069665bb51915078edc06d434cdf2a` —
40 items, 10 per class (trained_row / fresh_operands / variant_phrasing /
off_register), generated deterministically (`div0_heldout_freeze.py`,
seed 91) and **composed blind**: only exact-membership checks against the
corpus ran at generation time; no distance measure touched these items.
Dev-battery probes excluded from sampling. Frozen now rather than at
DIV-0 selection time because the scorer already exists and is in use —
every ad-hoc probe scored before a freeze erodes the §8 protection.

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
