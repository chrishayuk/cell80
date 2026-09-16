# NE-0b development: plasticity cadence

See the [fixed development plan](plan.md). This separate workspace imports the
sealed NE-0 cell rules and scheduler library. It screens update cadence for
operating stability on new development seeds. There is no prediction probe,
evaluation-seed mode, or acquisition claim in this study.

From the repository root:

```sh
cp experiments/neural-ecology-ne0b/dependencies.lock experiments/neural-ecology-ne0b/Cargo.lock
cargo test --release --locked --offline --manifest-path experiments/neural-ecology-ne0b/Cargo.toml
cargo build --release --locked --offline --manifest-path experiments/neural-ecology-ne0b/Cargo.toml
python3 -m unittest discover -s experiments/neural-ecology-ne0b/tests -v
python3 experiments/neural-ecology-ne0b/run.py fixtures
python3 experiments/neural-ecology-ne0b/run.py seal
```

Commit the source and generated `implementation-seal.json` before running:

```sh
python3 experiments/neural-ecology-ne0b/run.py screen
```

Metal access is mandatory. The generated C++ CPU backend is not CUDA hardware.
Each run independently advances checked-i64 reference, generated CPU, Metal and
repeated Metal worlds and compares full committed state, currents and deltas on
every tick. Reset checks use weights-only serialization. This development screen
does not repeat NE-0's fresh-process probe tests because it has no probe.

The driver refuses to overwrite any started screen. It retains all process exits
in `artifacts/attempts.jsonl`; full checkpoints, per-tick hashes and compact phase
summaries are in `artifacts/seed-SEED-k-K/`. `artifacts/report.json` contains all
twelve rows, fixed-rule candidate selection, and artifact hashes. Full outputs
are ignored by Git; compact development findings are tracked separately.

Independent candidates run in three processes. Timing includes host routing,
comparisons, hashing and checkpoint I/O under shared-device contention. It is not
an isolated backend benchmark. A passing candidate still requires a separately
registered learning experiment and fresh evaluation seeds.
