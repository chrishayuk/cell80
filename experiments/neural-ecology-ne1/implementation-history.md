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

Development uses the sealed serial driver. The initial evaluation wrapper used
eight independent single-BLAS-thread CPU workers and a shared lock serializing
GPU collection. The machine reports 16 logical CPUs and 128 GiB RAM.
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

Before evaluation, four isolated full-stream replays of the already measured
development time-zero fit branch were run concurrently. Every feature/stream
hash matched the original, with reference/CPU/Metal/repeated-Metal agreement,
reset and weight integrity retained. Total wall time was 22.19 seconds; the
records are in `scheduling-check.json` and `artifacts/scheduling-check-*`.
Based solely on this scheduling check, the evaluation wrapper now uses ten
single-BLAS-thread fitting workers (one per seed) and a four-slot GPU semaphore.
This avoids a second batch of CPU fits and preserves isolated trajectories and
the all-seeds fit/test barrier. Numerical settings and development code are
unchanged. The scheduling check precedes evaluation data and the evaluation seal.
The independent auditor also reconstructs all synthetic-control predictions and
their acceptance gates from saved coefficients.

## Completed development validation

The unchanged development run completed all 450 logical test rows and 1,065
logical candidate records. All score rows are valid; all 16 synthetic controls
pass. The runner audit verified the fit barrier, saved trajectories, code seal,
candidate selections and replayed predictions. The separate numerical auditor
reproduced all 255 distinct neural prediction arrays, covering all 450 rows,
checked 18 distinct fit normalizations and 135 model selections, and reproduced
all 300 contrasts, four fitted baselines and 16 synthetic-control predictions.
No solver retries, reader changes or development seed replacements occurred.

`summarize.py` emits all 450 descriptive groups and 15 arm/time context rows for
development. It requires completed results and the independent audit, retains
invalid counts, and does not summarize incomplete groups over a selected subset.
Feature moments are fit-only; activity is reported separately by stream. Weight
movement is descriptive context, not a measure of predictive alignment.
