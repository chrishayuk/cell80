# Deterministic Ecology: findings

Companion to `deterministic-ecology.md` (the design/pre-registration doc for EX-0…EX-5).
That doc says what each experiment would need to show; this one reports what running them
actually showed, with the receipts, one `##` section per experiment as they land — mirroring
the single multi-experiment design doc rather than one findings file per experiment.

Code lives inside `experiments/cell80-life/` (`src/rng.rs`, `src/genes.rs`, `src/history.rs`,
`src/ex0.rs`, `tests/ex0_*.rs`) rather than a new crate — deliberate, per the project's
current preference to stay inside `experiments/` rather than promote to a new workspace
member while this stays speculative/off-roadmap.

## EX-0 — the replay gate

**TL;DR: both gates passed on the first real run.** The same `(seed, genome)` run twice on
the CPU reference interpreter produces byte-identical history. The same run on the CPU
reference interpreter and on the Metal GPU body also produces byte-identical history —
same per-tick organism positions/energies, same mutation-RNG draws, same food array, same
summed IR-step cost, every tick, for all 200 ticks. Nothing here disagreed once and had to
be debugged; both `cargo test` assertions passed cleanly against the first working
implementation.

### What was built

- **`rng.rs`** — `draw(seed, tick, organism_id, stream) -> u32`, a SplitMix64-style
  finalizer over the four inputs. Unit-tested for the actual property a GPU dispatch needs:
  computing draws for a fixed `(seed, tick)` in ascending organism-id order vs. a different
  order produces an identical id→value map (`order_independent_across_organism_ids`).
- **`genes.rs`** — loads a stdlib gene cell once and runs it on two bodies: a fresh
  `cell80_core::Interp` per call (the CPU reference oracle — the same lowering pipeline
  already proven bit-exact in `gpu_cells.rs`/`msl_battery.rs`), and, on macOS, a compiled
  `rustmsl::GpuBatch` (the "one cell × N inputs" layout). Both read the *same* lowered IR.
- **`history.rs`** — a canonical, id-sorted, fixed-width-byte (not JSON) per-tick record
  folded into a running SHA-256: every living organism's id/pos/energy, its mutation draw,
  the food array, cumulative births/starved, and the tick's summed IR-step cost.
- **`ex0.rs`** — the tick engine. Every gene call is computed from an immutable tick-start
  snapshot (never from another organism's this-tick output), so each stage (decay, sense,
  hungry-check, eat, repro-check, split) runs as one batch across every living organism —
  a per-organism `Interp` loop for the CPU-reference engine, one `GpuBatch::run` dispatch
  for the GPU engine.
- **`tests/ex0_cpu_replay.rs`** (no platform gate) and **`tests/ex0_gpu_vs_cpu.rs`**
  (`#![cfg(target_os = "macos")]`) — the two assertions themselves.

### Receipts

Run parameters: grazer genome (`genomes/grazer.json`), seed `0x5eed_1234_c311_80ff`, 8
initial organisms, 24-tile world, 200 ticks.

| | value |
|---|---|
| CPU-reference history hash | `4d2d423d204d305fdc233d517b2ec69638b2338a662f8c13ac184b1beee55ca1` |
| GPU history hash | *identical to the above* |
| ticks recorded | 200 (no extinction) |
| final population | 560 |
| births (cumulative) | 552 |
| starved (cumulative) | 0 |
| summed IR steps, last tick alone | 25,200 |

Both `cargo test -p cell80-life` (all platforms) and the macOS-only GPU test are green;
`cargo clippy -p cell80-life --all-targets` is clean.

### A real, notable difference from `cell80-life`'s original dynamics — expected, not a bug

560 organisms after 200 ticks is wildly different from the original binary's steady ~8–12
under the same grazer genome (`cell80-life-findings.md` Finding 1). This is the direct,
expected consequence of EX-0's food-tile-eating simplification (**non-exclusive within a
tick**: every organism that intends to eat at a tile gets the snapshot's full food amount,
not a share of it, and the tile clears once at tick end) — necessary because "who ate
first" isn't a well-posed question once a tick is a batch dispatch rather than a
sequential `Vec` loop with a defined processing order. Once several organisms converge on
the same food-rich tile, each gets a full, uncontested meal every tick, so reproduction
compounds instead of being resource-limited — a materially different resource-contention
rule, not a materially different *mechanism*. **EX-0 was never scoped to reproduce
`cell80-life`'s exact population curve** — that fidelity claim belongs to EX-1 ("port the
grazer/rapid_reproducer genomes unchanged"), which does need to either resolve this
contention rule properly (e.g. split a tile's food across everyone who ate there that
tick) or explicitly re-derive the qualitative-regime comparison under the new rule.
Flagging this now rather than letting EX-1 discover it as a surprise.

### What this shows

- **EX-0's two literal gates pass**: replay is bit-exact (same seed twice → identical
  history), and GPU ≡ interpreter is bit-exact (same run, two bodies → identical history),
  including the mutation-RNG draws, which were exercised for real (not deferred) and shown
  order-independent by both the targeted unit test and by matching across two engines with
  different (undefined, in the GPU's case) internal dispatch/iteration order.
- The determinism spine doc 17 §1 requires — world state as a pure function of
  `(seed, genome-set, tick)`, counter-based RNG, GPU≡interpreter parity, content-addressable
  execution — holds for the scope tested: one homogeneous genome, no species, no
  mutation-driven diversity, the existing 1D world.

### What this does *not* show (deferred, by design — see `deterministic-ecology.md`)

- **Heterogeneous-genome GPU batching is still unsolved.** A compiled Metal module shares
  one `n_inputs` across every cell in it (confirmed in `rustmsl/src/codegen.rs`); EX-0
  sidesteps this by running exactly one genome for the whole population. EX-2's
  bytecode/genome-diversity mutation will need either a real solution here or an explicit
  documented workaround (e.g. per-genome-variant dispatches, weighed against the ~140 ms
  fixed per-launch cost noted in `docs/14-model-native-cells-spec.md`).
- **No shared mutable world on the GPU.** Every gene call this tick reads only an
  organism's own state plus an immutable tick-start snapshot; there is no on-GPU food grid
  or cross-organism lookup. This is fine for a grazer-only world with no predation, but
  EX-3 (predator/prey) will need cross-organism sensing that this architecture doesn't
  provide — it stays a host round-trip (dispatch → readback → host computes next
  input → re-dispatch) until/unless that changes.
- **RNG is a host-side Rust function, not a `.cell`.** Chosen for EX-0 because the doc's
  contract is about the RNG's mathematical shape (pure function of seed/tick/id/stream),
  not which body executes it, and authoring a new cell-family for this wasn't justified by
  EX-0 alone. Promoting it to a `.cell` (so a mutation-decision draw happens inside the
  sandboxed genome execution itself, not host-side) is a real, named option if EX-2 needs
  it — not decided here as better or worse, just not yet done.
- **Single-homogeneous-genome-only scope.** No species, no predator, no mutation-driven
  genome diversity within a run — all EX-1/EX-2/EX-3's job.
- **The food-tile-contention divergence from `cell80-life`'s exact tick semantics**,
  covered above — a stated, deliberate simplification for making CPU-reference and GPU
  comparable, not a claim about `cell80-life`'s original population dynamics.
- **The IR-step signal recorded per tick is a single summed total across every organism and
  gene-role call, not a per-organism-per-role breakdown.** Sufficient to catch a
  GPU/interpreter step-count divergence, but a real divergence localized to one organism/
  role would need re-running with a per-call trace, not just re-reading this history.

### Reproduce it

```
cargo test -p cell80-life --test ex0_cpu_replay      # any platform
cargo test -p cell80-life --test ex0_gpu_vs_cpu       # macOS (Metal) only
cargo clippy -p cell80-life --all-targets
```

### What would raise confidence further

- Re-run with several more seeds (this doc reports one) to confirm the CPU-reference/GPU
  agreement isn't a coincidence of this particular seed's mutation-draw sequence.
- A per-organism-per-role IR-step trace (not just a per-tick summed total) for the case
  where a future change causes a genuine disagreement, so it localizes immediately.
- Resolve the food-tile-contention question before EX-1 relies on this engine for a
  population-curve comparison against `cell80-life`'s original findings.
