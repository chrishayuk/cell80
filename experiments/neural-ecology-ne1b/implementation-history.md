# NE-1b implementation and execution history

## Initial development implementation — 2026-09-18

Specification commit: `b6ec99f`. The specification, selectors and static inventory
remain unchanged. No intervention neural measurements preceded the development
implementation seal.

The separate Rust runner batches 71 independent systems, uses the inherited
current calculation and executable neuron cell, and omits the disabled plasticity
path. Each tick compares reference, generated CPU, Metal and repeated Metal state.
It saves q arrays, full-state feature hashes, per-tick node hashes, persistent
hashes and activity. A fixture explicitly compares the generalized frozen-weight
step to the unchanged three-system NE-0c step for 32 ticks. All inherited GPU
fixtures match.

The Python runner imports the sealed NE-1a numerical instrument unchanged.
Distinct fit/validation feature hashes identify cacheable duplicate fits; all
logical branches remain in the output. Identity branches execute anew and must
match archived trajectories, normalization, candidate coefficients, selection and
prediction bytes. Eight CPU fit workers and at most three concurrent GPU
collection jobs change scheduling only. The global fit manifest precedes all
intervention test trajectories.

Seven implementation tests pass. Both required development synthetic controls
were refitted and passed before neural validation. The initial sandboxed fixture
command reported no Metal device; rerunning with GPU access passed. This was an
environment-access failure before neural measurement, not a scientific failure.

The independent auditor does not import the runner or measurement instrument.
It uses the previously independent NE-1a prediction arithmetic and independently
reconstructs subset reversions, fit normalization, candidate objectives/gradients,
selection, test metrics, calibration, identity predictions and every contrast.

`development-seal.json` pins code, compiled binary, dependencies, inherited
artifacts and calibration evidence. Its commit precedes development trajectories.
Development must complete and pass its independent audit before evaluation sealing.

## Development bookkeeping revision — before test or evaluation

The initial development attempt was stopped before fitting/test after static
review found that the pre-test manifest would include a per-seed command log
which test execution would later append to. No scientific outcomes motivated
the change. Partial fit/validation trajectories and console output are retained
under `artifacts/development-attempt-v1/`; the original runner and seal are also
preserved in `implementation-v1/` and commit `619d95b`.

The correction places command logs in `artifacts/attempts/`, outside the seed
directories covered by the fit manifest. A regression test enforces that path
separation. The scientific specification, selector, neuron evaluator, numerical
instrument, calibration evidence and binary are unchanged. Development is
restarted from zero under a new committed implementation seal; no v1 trajectories
are reused as completed validation evidence.
