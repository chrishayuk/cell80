# NE-1a implementation history

The numerical specification was committed at `3183b19` before implementation or
NE-1 neural data. Its scientific choices have not changed.

## Synthetic calibration and engineering

The new float64 instrument passed analytic-gradient, intercept-penalty,
constant-feature transport, indexing, routing, tie and failure-policy tests.
Its source was then hashed before all 80 development synthetic candidate fits.
All 16 selected controls passed. Full calibration records and the original code
seal are preserved under `artifacts/calibration-dev-3001-v1`; a compact report is
[calibration-results.json](calibration-results.json).

The initial neural-runner build attempted to call a private activity helper in
NE-0c. The new runner was corrected to update that helper's public counter fields;
no inherited source changed. The Rust lockfile was based on NE-0c's lockfile and
the final new dependency set is recorded in `dependencies.lock`. A reporting
fixture initially assumed its temporary directory was inside the repository;
the fixture now supplies its temporary repository root explicitly. All eight
tests pass. All 700 inherited CPU/Metal fixtures match their original hash.
These corrections preceded every NE-1 neural trajectory.

## Phase barriers

As required by the execution sequence, development synthetic calibration is
completed first, with all sixteen fits locked before its test symbols are
generated. Development neural fitting has a separate barrier before its test
**trajectories**. The seed-3001 test symbols are already known from the mandated
synthetic calibration; no neural test arrays or predictions are available at
that point. This order is disclosed explicitly rather than describing those
development symbols as unseen.

Evaluation has the strict global barrier: no evaluation test symbols or neural
test trajectories are generated until every neural and synthetic fit/selection
for all ten evaluation seeds is complete and included in the fit manifest.
No evaluation stream may begin before successful development validation and a
committed evaluation implementation seal.

Exact duplicate arrays and model inputs can share a computation after equality
checks. The full logical score/candidate matrix is retained, including invalid
fits. NE-1b remains deferred until the full NE-1a matrix and audit are complete.

## Evaluation scheduling wrapper (before evaluation)

Development uses the sealed serial driver. A separately sealed evaluation wrapper
can use eight independent single-BLAS-thread CPU workers, with one shared lock
serializing GPU collection. The machine reports 16 logical CPUs and 128 GiB RAM.
This wrapper changes scheduling only and calls the same per-seed functions.
All ten fits/selections still complete before the global test manifest is written
and any evaluation test stream is generated.

Its isolated barrier test initially caught a legacy import-path collision:
`import run` could select NE-0's driver after importing the inherited probe.
The wrapper now loads NE-1's driver by its explicit file path. The barrier test
passes without generating evaluation data. This additional wrapper and test are
outside the development seal and must be included in the evaluation seal before
use; the running development implementation is unchanged.

A separate `audit_saved.py` was also added before development test scoring. It
imports neither the fitting instrument nor the runner, and independently rebuilds
fit normalization, lambda selection, test predictions, transported-model identity,
paired contrasts and fitted baselines from saved arrays. Its two hand-calculated
fixtures pass. It makes a same-environment numerical reproduction claim, not a
cross-library claim. This audit and its tests will be included in the evaluation
seal; they do not change the active development seal or any measurement.
