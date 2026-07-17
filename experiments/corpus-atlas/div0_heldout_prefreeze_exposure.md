# Pre-freeze scorer exposure — disclosed appendix to the §8 held-out freeze

Companion to `div0_heldout.json` (frozen 2026-07-17, sha256 `6549a2cf…`,
commit 85e713b). The freeze stops the erosion of the forking-paths
protection; this appendix documents exactly how much erosion there was, so
the confirmation set's eventual verdict is unimpeachable without anyone's
word for it. Committed immediately after the freeze commit in the same
session; the held-out generator ran blind (membership checks only) and its
items are verified disjoint from everything below.

## Externally-composed texts fed to a distance scorer before the freeze

1. `"What is 25 multiplied by 32?"` — surface-index smoke3 probe
   (`atlas_surface.py`). Also DIV-0 dev probe D-B5; dev items are the
   selection set, so exposure is by design.
2. `"Once upon a time, there was a little girl"` — surface-index sanity
   query.
3. `"One day, a little girl named Zorblax found 7 shiny pebbles."` —
   two-distance demo (surface + skeleton smokeD).
4. The six DIV-0 dev probes (D-B1, D-B2, D-B3, D-B5, D-B6d, D-B6w) —
   scored with metrology's coverage machinery under skeleton-v1
   renderings in the equivalence audit (`skel_v1_equiv_check.py`,
   c9de508). Dev battery = the selection set; exposure is its role.

## Corpus-internal material queried (not composed probes)

- Surface smoke1/smoke2: raw token slices of training chunks (verbatim
  and boundary-straddle checks).
- Skeleton smokeA/smokeB: chunk-1000 skeleton slice; sentinel-straddle
  slice.
- Skeleton smokeC: 500 random stream positions (alignment round-trip).
- Digit-piece corpus positions decoded for receipts
  (`retro_digit_prior.py` — counting, no distance scoring).
- The full CN-7 corpus rendered for cardinality counting in the
  equivalence audit (counting, not distance scoring).

## Disjointness

No held-out item text appears above; verified mechanically at appendix
time (exact string comparison, all 40 items vs the composed-probe list).
The held-out classes were generated from templates and seeds, not from
any text previously scored.
