# Corpus Atlas — findings (session 2026-07-17)

Companion to the instrument spec (`corpus-atlas-DRAFT.md`), which carries the
design and the inline result blocks. This document is the standalone writeup:
what was built, what was found, and the registered-prediction scoreboard.
All artifacts live in `experiments/corpus-atlas/`; large data (streams,
catalogue) is gitignored under `artifacts/` and reproducible from the
committed scripts + recorded hashes (see §7).

## What the atlas is

A queryable, sha-fenced index over v11's 24M-token pretrain that answers, for
any probe text, how far it sits from the training manifold at two levels —
**surface** (exact token sequence) and **skeleton** (frame, with digits→D and
names→N) — each with **receipts** (the nearest training rows themselves). The
purpose is to report generalization as a function of *measured* training
distance rather than as a design label. Built this session as five in-repo
components; all smokes green, every input that could drift refuses on a hash
mismatch.

## What was built (all verified green)

| component | file | state |
|---|---|---|
| Stream materialization (gate A0) | `a0_dump_stream.py`, `a0_mask_check.py` | 16M+8M tokens, determinism + training-era identity verified |
| Surface index | `atlas_surface.py` | per-phase suffix arrays, longest-match + receipts |
| Skeleton index + alignment map | `atlas_skeleton.py` | skeleton-v1, alignment round-trip 100%/500 |
| Two-distance scorer | `atlas_skeleton.py score` | surface + skeleton pair, receipts on both |
| Midtrain wall-checker | `midtrain_shards.py` | 4 shards, FS-bank admission gate |

Structural fences (all confirmed by tamper/round-trip tests): the query path
refuses a tokenizer, spaCy model, or shard source whose sha256 differs from
the index fingerprint; "seen" is defined within a 256-token training chunk
(inter-chunk sentinels), so matches never straddle a boundary the model never
attended across; the alignment map is *constructed* from original pieces, not
recovered by decode→re-encode.

## Gate results

- **A0 — PASS.** The pretrain does not exist as a file; it is streamed
  (`roneneldan/TinyStories`, 16M tokens @ seed 42 + 8M @ seed 43). Dumped
  twice with identical per-phase sha256; env + HF revision pinned; dataset
  last-modified (2024-08-12) predates training (2026-04-11). Reproducing
  `v11_train_mask.pt` bit-for-bit from the reconstructed stream order
  established that the reconstruction *is* the training stream, not just a
  self-consistent dump.
- **A1 — caught a real conflation on first run.** The v11 SP id space is
  71,261 pieces; the "8,599" in prior notes was the train-mask *whitelist*,
  not a trained-ids census. The true exercised set is 17,158 distinct ids.
  Dumps are uint32 accordingly.

## Audits (three; nothing frozen flipped)

- **Mask-as-census audit.** The train mask is a decode whitelist, not a
  trained-ids set. Recompute of CN-1 §8.1's "29.8% vs 12.1% chance" against
  the true census: 32.0% vs 24.1% — the "2.5×-chance" flavor was a whitelist
  artifact; the 68%-never-trained verdict is unchanged. Nothing graded flips.
- **Normalizer equivalence audit** (`skel_v1_equiv_check.py`). skeleton-v1
  (spaCy) is **NOT equivalent** to DIV-0's metrology-M1 (deterministic
  lexicon): on the frozen CN-7 corpus, v1 inflates S2 to 31 distinct surfaces
  vs the pinned 12 (37 frames diverge, all v1-upward), from spaCy noise on
  template text (name-tag instability, span swallowing, symbol mistags in
  call grammar). **Consequence, pinned:** metrology-M1 keeps authority for
  DIV-1 audit rows and the wall-checker; skeleton-v1's domain is the
  natural-prose pretrain, cardinalities read as mild upper bounds.
- **Digit-prior retrodiction** (`retro_digit_prior.py`) — the atlas
  retrodicting CN-10's behavioral smoke. Global unigram vs median digit rank:
  Spearman −0.37 (weak). Conditioning on the smoke's actual quantity
  (number-initial position): **−0.77, and −0.87 excluding the frame-bound
  "3 year old" idiom.** Two independent instruments (corpus counting,
  behavioral readout) agree on what the digit prior *is* — the two-routes
  rule crossing an experiment boundary. Ruling: CN-10's operative
  delta-from-prior correction stays the behavioral shuffled-rank; corpus
  count is its validating receipt, not its replacement.

## Harvest and the DIV-1 calibration (the loaded finding)

`harvest_catalogue.py` → `harvest_summary.json`. 1,360,050 distinct v1 frames
/ 1,308,777 m1 frames over the stream; the two renderers agree on the
distribution to three decimals (hapax 0.9398 vs 0.9396) — two-routes passed on
the harvest itself.

**The pretrain's frame-diversity distribution is extreme-Zipf: 94% of frames
occur exactly once** (p50 = p90 = 1, p99 = 5); the head is a few story
formulae ("once upon a time, there was a little girl named N" — 9,019×, 185
fillers). DIV-1's seeded cardinality levels land as percentiles: **1 = the
modal frame (94th), 8 = 99.5th, 64 = 100th, and 512 exceeds the pretrain's
maximum observed filler diversity (303 v1 / 331 m1).** DIV-1's top arm is
therefore *super-corpus*, and "64 is typical diversity" is refuted — 64 is
already head-of-distribution.

## Registered-prediction scoreboard (register-then-run, in git in order)

The 94%-hapax finding pressures the diversity law: a fluent compiler emerged
from a corpus of mostly-singleton frames. Two predictions were registered
*before* computing, both at honest thresholds:

- **Reg 1 — short-frame rescue (proposed chat-side by Chris, relayed):
  FAILED.** Predicted ≤8-symbol frames would collapse hapax below 0.5;
  actual **0.8637**, p50 = 1, DIV-1 level 8 still at the 98.6th percentile
  even among short frames. Falsifier branch fired.
- **Reg 2 — sub-sentential n-gram ledger: primary INDETERMINATE, secondary
  FALSIFIED.** Token-weighted 4-gram coverage@64 = **0.3306** (inside the
  disclosed 0.25–0.50 indeterminate band; predicted >0.5); 8-gram coverage@8
  = **0.0909** (below the 0.15 falsifier line). Repetition mass decays fast
  with scale: majority-repeated at (n=4, T≥8) = 0.559, a third at T≥64,
  wasteland by clause scale.

Both misses recorded under standard scoreboard rules, credited to originators.

## The adopted resolution

Of three candidate reformulations, threshold-lowering and unit-shrinking are
**rejected** (each retreat is available after every miss — the regress
problem). Adopted: **the diversity law governs marshalling, not fluency —
its correct original scope, not a retreat.** The hapax finding is ordinary
linguistic productivity (fluency = manifold interpolation over the
n=4-repetition / n=8-wasteland compositional regime); the law's evidence base
was always *binding* failures (all five walls; CN-7's S2-at-cardinality-2
already showed fluent call-format with broken operand binding). **Dissociation
prediction, pinned verbatim for R1.1-side registration before any DIV-1 arm
exists:** across the 1/8/64/512 arms, marshalling (P-m) moves strongly with
frame diversity while fluency (replay NLL, paraphrase) barely moves; three-way
grading includes both failure branches, making the scoped law *more*
falsifiable than the original.

## Last corpus experiment — the midtrain wall-checker

`midtrain_shards.py` → `cn8_band_midtrain_distances.json`. The frozen CN-8
eval bands scored against each arm's own training shard, committed before any
grading verdict. The predicted two-distance signature confirmed:
**M1-skeleton saturated** (all 51,031 arm-B rows and 346,962 A-tok rows share
the bands' "N + N =" skeleton, identical across bands) and **surface-graded
monotone** (B0→B1→B2 max-match mean 8.85 → 7.56 → 7.45 on the arm-B shard;
cn7_train flat ~6.5). The surface-distance drop concentrates at B0→B1 —
**co-located with where exact accuracy collapsed 1.000 → 0.000.**

Two fences on that result: (1) it is **three points and a cliff, not a
curve** — band histograms overlap at the boundary, no item-level pass/fail
variance, so distance separates bands only in the mean; the real P-m curve
needs DIV-1's arms or a finer ladder. (2) **Cross-shard comparisons are
unnormalized** — max-match scales with shard size (A-tok 347k rows vs B 51k),
so cross-shard reads await a size-matched null.

## Convergence

Three independent instruments this session point the same way — the fragile
thing is **binding/marshalling**, not computation or fluency: the diversity
ledger (fluency cheap on singletons), the atlas band distances (failure at the
surface-distance cliff, frame distance zero), and CN-8's own grader fields
(`cn8_pregrade_observations.md`: arm B `col_cond_correct = 1.000` with
`index_ok = 0.000` — algorithm perfect, operand transcription broken beyond
the trained band, both seeds).

## Provenance and storage status

Large data (corpus `.jsonl`, `.u32` streams, 277 MB catalogue) was purged from
git history this session and now lives gitignored under `artifacts/`. This is
*stronger* provenance, not weaker: each is reproducible from a committed
generator (`cn7_corpus.py`, `a0_dump_stream.py`, `harvest_catalogue.py`) with
its sha256 recorded in tracked manifests. The DIV-0 anchor holds — on-disk
`cn7_corpus_train.jsonl` is still `4e146e6d…`, byte-identical to the frozen
provenance.

## Open / handed off

- **R1.1-side:** register the dissociation prediction (wording pinned in the
  DRAFT §"Pinned reading"); FS-bank freeze (admission machinery ready via
  `midtrain_shards.py check`).
- **Atlas builds remaining:** level-3 model-side distance; mean-per-position
  band refinement (dynamic range where max-match compresses to 7–11);
  size-matched cross-shard null; an atlas m1-mode renderer so DIV quantities
  compute atlas-side under the authority-holding normalizer.
