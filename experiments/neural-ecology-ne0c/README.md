# NE-0c: confirmatory learning at K=4096

The [preregistration](preregistration.md) fixes fresh evaluation seeds 4001–4010,
the NE-0b-selected cadence, the unchanged NE-0 measurement procedure and G0–G6.
Continuation from final-A transient state is a descriptive secondary branch;
it cannot change the primary verdict.

The standalone Cargo workspace imports the unchanged `ne0` library for cells,
compilation, mathematical rules and state types. `src/runner.rs` is an explicit
fork of the sealed NE-0 runner with cadence eligibility, a reset switch, and
initial-state instrumentation. `analyse.py` changes the cohort and separates
secondary reporting; it calls the original probe's exact statistical function.
The original probe, replay validator and archiver are used directly.

```sh
cp experiments/neural-ecology-ne0c/dependencies.lock experiments/neural-ecology-ne0c/Cargo.lock
cargo test --release --locked --offline --manifest-path experiments/neural-ecology-ne0c/Cargo.toml
cargo build --release --locked --offline --manifest-path experiments/neural-ecology-ne0c/Cargo.toml
python3 -m unittest discover -s experiments/neural-ecology-ne0c/tests -v
python3 experiments/neural-ecology-ne0c/run.py fixtures
python3 experiments/neural-ecology-ne0c/run.py dev
python3 experiments/neural-ecology-ne0c/run.py seal
```

Commit the generated implementation seal, then run:

```sh
python3 experiments/neural-ecology-ne0c/run.py eval
```

Metal access and NE-0's pinned Python dependencies are required. Development
validation is restricted to seed 3001; its A checkpoint must exactly reproduce
NE-0b's selected run. No scientific parameters are fitted or tuned during that
validation. Evaluation requires all ten fresh seeds and unchanged committed seal
contents; three seed workers run independently.

Each seed runs A → primary B → fit → primary C → fresh-process replay/identity
checks → primary score → continuation B → separate fit → continuation C →
secondary score → lossless archive. B and C of the secondary each branch from
the same full final-A snapshot; they do not carry state from one another.
All weights are frozen in primary and secondary B/C. Initial-node hashes expose
the reset/continuation distinction directly.

`artifacts/attempts.jsonl` records starts and exits. Completed phases may resume
only with identical seed, tick count and binary. A started incomplete phase is
never overwritten; any rerun needs a new retained attempt and explicit record.
Raw outputs are ignored by Git; compact reports and provenance are tracked.
Inherited archival verifies a decompression hash before replacing raw duplicate
diagnostics and tick/checkpoint logs. Primary arrays/probes stay directly readable.
