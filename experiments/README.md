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
