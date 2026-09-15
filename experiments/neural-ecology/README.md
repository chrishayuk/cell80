# NE-0: local adaptation and temporal prediction

Implementation of the [sealed v1 protocol](../neural-ecology-ne0-preregistration.md).
This directory is a standalone Cargo workspace. It uses existing Cell80 compilation
and execution APIs; it adds no recurrent primitive to `CellGraph`.

## What runs

- `cells/node.rs`: bounded integrate-and-fire state update.
- `cells/plasticity.rs`: the registered local timing rule.
- `src/model.rs`: independent checked-i64 oracle, addressed RNG, graph construction,
  canonical state, and explicit persistent-only checkpoint type.
- `src/backend.rs`: Cell80 IR interpretation, Cell80-generated C++ on the CPU,
  and Cell80-generated MSL on Metal. The CPU bridge reuses the CUDA vocabulary
  emulation approach of `rustmsl::cpu_emu`; it makes no CUDA hardware claim.
- `src/runner.rs`: independent worlds for reference, generated CPU, Metal and
  repeated Metal; equality after each phase and hashes at every tick.
- `probe.py`: fixed-budget float64 softmax instrument, conditional shuffle,
  exact sign tests, and G0–G6 reporting.
- `run.py`: phase sequencing, completed-phase resume, attempt journal and seal.

Metal is mandatory for primary execution. A missing device is an error, never a
CPU-only pass. Probe optimization changes only readout coefficients; it consumes
frozen trace arrays exported by the experiment and cannot update neural weights.

## Build and verify

From the repository root, with the pinned Python packages available:

```sh
cp experiments/neural-ecology/dependencies.lock experiments/neural-ecology/Cargo.lock
cargo build --release --locked --manifest-path experiments/neural-ecology/Cargo.toml
cargo test --release --locked --manifest-path experiments/neural-ecology/Cargo.toml
python3 -m unittest discover -s experiments/neural-ecology/tests -p 'test_*.py' -v
experiments/neural-ecology/target/release/ne0 fixtures experiments/neural-ecology/artifacts/logs/fixtures.json
```

`dependencies.lock` is a committed copy of Cargo's resolved lockfile (the parent
repository ignores files named `Cargo.lock`). The numerical environment is pinned
in `requirements.txt`; the run records actual Python, NumPy and numerical-library
versions. BLAS thread limits are set to one both through environment variables
and `threadpoolctl`.

## Development and evaluation

```sh
experiments/neural-ecology/target/release/ne0 experiment dev experiments/neural-ecology/artifacts
```

Development permits only seeds 1001–1003. `--seeds 1001` can finish one development
replicate; subset reports have different names. Each seed executes A, B, fits and
freezes probes, then executes C. Five fresh-process replays check persistent B/C,
initial-weight restoration B/C, and disabled adaptation A. The driver preserves
attempt records and refuses to overwrite a started phase. An interrupted phase
without its completion summary needs an explicitly recorded rerun in a new output
directory; only completed phases resume automatically. Checkpoints are retained,
but this version does not implement mid-phase restart.

Evaluation schedules three independent seed processes concurrently. Every seed's
A → B → fit → C sequence stays ordered; its numerical library uses one thread.
Topology, inputs, updates and files are private to a seed. This is host scheduling
of independent replicates, not a change to neural scheduling or a pooled dataset.
Wall times under contention are recorded as such and are not isolated-backend
benchmarks.

After all development verification and source checks are complete:

```sh
experiments/neural-ecology/target/release/ne0 experiment seal experiments/neural-ecology/artifacts --seal experiments/neural-ecology/implementation-seal.json
```

The seal records the binary, protocol, source/dependency files, emitted kernels,
fixture evidence, development inventory and environment. Commit the seal before
evaluation. The evaluation entry point verifies the committed seal and all
recorded file hashes. It permits only seeds 2001–2010; the driver requires all ten.

```sh
experiments/neural-ecology/target/release/ne0 experiment eval experiments/neural-ecology/artifacts --seal experiments/neural-ecology/implementation-seal.json
```

A development activity failure is reported; parameters are not automatically
tuned. Evaluation is allowed to return a bounded negative result. Execution or
persistence verification failures prevent sealing. No development score counts
toward confirmatory inference.

## Representation and replay details

The protocol's arithmetic is defined by the checked-i64 reference. Cell80's
current scalar dialect uses narrower lanes. The compiled implementation is an
exact bounded refinement: current is in [−512,560], leaked potential plus current
is in [−512,783], trace multiplication is at most 3,825, trace update before its
cap is at most 303, and the plasticity numerator is in [−255,255]. Validated inputs
therefore cannot overflow the cell operations. All reductions and persistent
weight updates use checked i64 on the host. No numeric constants or rounding rules
are changed by this representation.

Canonical node snapshots are six bytes: `v:u16,r:u8,s:u8,q:u16`, little-endian.
Cell80's ABI uses four two-byte slots, so an explicit eight-byte ABI adapter pads
the two byte fields with zeros. Those padding bytes are checked on return and
never enter the canonical model state. `PersistentCheckpoint` contains only
weights, seed and immutable topology; it cannot serialize neuron history and
rejects unknown fields. Full periodic checkpoints separately retain transient
state for debugging.

On macOS, large SHA-256 records use the system CommonCrypto implementation;
portable SHA-256 fixtures check identical digests. This changes hashing cost,
not the protocol's canonical byte sequence. Each backend still computes its own
state digest. Neural comparisons are exact; the CPU float64 probe is pinned to
its recorded environment rather than advertised as cross-platform bit-identical.

## Outputs and limitations

Disposable outputs live under `artifacts/`; compact findings and the implementation
seal are tracked separately. Phase summaries include activity, endpoint fractions,
requested/realised/clipped movement, exact final hashes, duration and (in the final
runner) peak process RSS. RSS includes host allocations and mapped shared buffers;
it is not a separate measurement of all Metal driver/device memory. Probe costs
are reported separately. Development timings from different runner builds must
not be pooled as a performance benchmark.

After a seed's checks and scoring complete, `archive.py` losslessly compresses
tick histories, periodic full snapshots and duplicate diagnostic arrays. Each
compressed file is decoded and SHA-256 checked before it replaces its raw file;
`logs/archive.json` records both hashes and sizes. Primary measurement arrays,
probes, summaries and persistent checkpoints remain directly readable. Use
`gzip -dk FILE.gz` to restore a raw file before manually repeating a diagnostic
that reads that history. No experimental state is discarded by compression.

The first development adaptation began before the persistent-only serializer and
accelerated hashing were finalized. Its original full checkpoints and launch
binary hash are retained. `extract-persistent` converts such a **development-only**
snapshot without changing weights; the original full snapshot remains. Subsequent
replays compare every tick and feature against that original execution. The
evaluation binary and serializers are sealed together before any evaluation data.

Known compiler limitation encountered during fixtures: 16-bit `as i16` casts lose
their signed width in the current lowerer, and local annotations do not restore
it. The edge cell passes the bounded numerator through an explicitly typed helper
parameter, preserving signed division. Fixtures include −1/64 → 0 and −64/64 → −1
on the independent oracle, IR, generated CPU and Metal. The core compiler was not
modified as part of this experiment.
