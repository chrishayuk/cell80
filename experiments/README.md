# Experiments

Experiment source, preregistrations, compact result records, and findings stay in
Git. Disposable run products belong in an `artifacts/` directory local to each
experiment:

```text
artifacts/
  datasets/     generated corpora and derived data
  checkpoints/  model and optimizer checkpoints
  logs/         captured stdout/stderr and service logs
  indices/      streams, arrays, and search indices
```

The Python experiment suites create these directories when they write an
artifact. Readers accept an explicit path and, for bare filenames, look in the
appropriate artifact directory before checking the legacy experiment root.

To remove all disposable experiment state:

```sh
find experiments -type d -name artifacts -prune -exec rm -rf {} +
```

Set `CELL80_ARTIFACT_ROOT` to place the current suite's artifacts elsewhere.
`CELL80_CELL_NATIVE_ARTIFACT_ROOT` and `CELL80_CORPUS_ATLAS_ARTIFACT_ROOT` are
suite-specific overrides. Typed subdirectories are still created below the
selected root.

## Neural ecology

[NE-0 preregistration](neural-ecology-ne0-preregistration.md) specifies the first
local-adaptation experiment: exact integer dynamics, temporal grammar, paired
controls, reset semantics, and G0–G6 decision rules. The protocol is locally
hash-sealed; the [implementation and run instructions](neural-ecology/README.md)
include backend replay checks and the evaluation seal. Verify the document from
the repository root with:

```sh
shasum -a 256 -c experiments/neural-ecology-ne0-preregistration.sha256
```

The [completed ten-seed findings](neural-ecology/findings.md) report passing
execution and persistence checks but unmet learning criteria for the registered
configuration. All results and gate outcomes are retained.

[NE-0b development](neural-ecology-ne0b/README.md) tests whether less frequent
application of the same plasticity rule can satisfy the operating-regime limits
on new development seeds. Its fixed grid does not use prediction scores or
constitute a new confirmatory learning experiment.

The [completed NE-0b development findings](neural-ecology-ne0b/findings.md) select
one update every 4,096 ticks as the only tested schedule meeting all operating
criteria across three development seeds. Predictive learning remains untested
in that development study.

[NE-0c](neural-ecology-ne0c/README.md) preregisters the confirmatory learning test
at K=4096 on fresh seeds 4001–4010. Primary post-reset prediction and a separate
descriptive no-reset diagnostic use the original fixed probe and controls.

The [completed NE-0c findings](neural-ecology-ne0c/findings.md) do not meet the
registered learning criteria across all ten seeds. Activity, adaptation and replay
checks pass, but Plastic does not improve on Frozen or Yoked by the registered
criteria. All primary and secondary results, the mechanical audit, and recovery
from a disk-space interruption are retained; the sealed implementation is unchanged.

[NE-1](neural-ecology-ne1/README.md) measures the archived NE-0c systems with
calibrated readouts and history decoding across five adaptation times. All 50
required snapshots are verified. Findings on these previously observed seeds
remain exploratory and require fresh-seed confirmation for a selected mechanism.
The [NE-1a numerical specification](neural-ecology-ne1/measurement-spec.md) is
frozen before measurements, including 4,500 neural score rows and explicit
calibration/convergence failure rules. All 16 synthetic controls and 700 inherited
CPU/Metal fixtures pass. [NE-1a completed](neural-ecology-ne1/findings.md) with
4,500/4,500 valid neural rows, all 160 calibration controls passing, and both
audits passing. Frozen already supports strong history decoding. Plasticity
changes reader alignment and improves some refitted readers, but does not give
a uniform next-symbol advantage over Frozen. NE-0c's verdict is unchanged;
NE-1b interventions remain unexecuted and require a separate frozen specification.
