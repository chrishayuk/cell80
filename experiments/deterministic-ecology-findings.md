# Deterministic Ecology: findings

Companion to `deterministic-ecology.md` (the design/pre-registration doc for EX-0…EX-5).
That doc says what each experiment would need to show; this one reports what running them
actually showed, with the receipts, one `##` section per experiment as they land — mirroring
the single multi-experiment design doc rather than one findings file per experiment.

Code lives inside `experiments/cell80-life/` (`src/rng.rs`, `src/contention.rs`,
`src/genes.rs`, `src/history.rs`, `src/pools.rs`, `src/composition.rs`, `src/lineage.rs`,
`src/ex0.rs`, `src/world2d.rs`, `src/ex1.rs`, `src/ex2.rs`, `src/predation.rs`, `src/ex3.rs`,
`src/bin/ex1_sweep.rs`, `src/bin/ex2_mutation_report.rs`, `src/bin/ex4_lineage_report.rs`,
`src/bin/ex3_predator_prey_report.rs`, `src/bin/ex3_arms_race_report.rs`,
`src/bin/ex5_soma_export_report.rs`, `tests/ex0_*.rs`, `tests/ex1_*.rs`, `tests/ex2_*.rs`,
`tests/ex3_*.rs`, `tests/ex4_*.rs`, `tests/ex5_*.rs`) rather than a new crate — deliberate,
per the project's current preference to stay inside `experiments/` rather than promote to a
new workspace member while this stays speculative/off-roadmap. EX-5 additionally reuses the
main `cell80` crate's multi-target RV32 export API (`Cartridge::compile_rv32`/`Rv32Runner`,
`docs/13-multi-target-spec.md`) read-only — no changes to `cell80`/`rustrv32`/`rustz80`.

## EX-0 — the replay gate

**TL;DR: both gates passed, and pass again after fixing a real bug this doc originally
reported.** The same `(seed, genome)` run twice on the CPU reference interpreter produces
byte-identical history. The same run on the CPU reference interpreter and on the Metal GPU
body also produces byte-identical history — same per-tick organism positions/energies,
same RNG draws, same food array, same summed IR-step cost, every tick. Both gates passed
cleanly against the first working implementation; this section was then updated once (see
below) after fixing the food-tile-contention bug the first version of this doc flagged as
a known, expected divergence — EX-1 needed it fixed, and fixing it in `ex0.rs` directly
(rather than only in EX-1's 2D engine) meant re-verifying both gates still held, which they
did without any test changes.

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
- **`contention.rs`** (added after the fix below) — `resolve_eat_contention`: among
  organisms that both chose "eat here" and passed `hungry_promoter` at the same tile this
  tick, the one with the lowest `rng::draw(seed, tick, organism_id, EAT_CONTENTION_STREAM)`
  wins the tile outright; everyone else keeps their post-decay energy. Order-independent by
  construction (unit-tested directly: same winners regardless of candidate-list order).
  This is the RNG's first real, branching use — EX-0's original mutation draw was computed
  and recorded but never decided anything.
- **`tests/ex0_cpu_replay.rs`** (no platform gate) and **`tests/ex0_gpu_vs_cpu.rs`**
  (`#![cfg(target_os = "macos")]`) — the two assertions themselves.

### Receipts

Run parameters: grazer genome (`genomes/grazer.json`), seed `0x5eed_1234_c311_80ff`, 8
initial organisms, 24-tile world, 200 ticks. **Post-contention-fix numbers** (superseding
the first version of this doc, which reported the bug's numbers as the baseline — see
below for both):

| | value |
|---|---|
| CPU-reference history hash | `32945ddd8f96f34a265c43e750658590917e143b985e24a2de65cd095ff0a747` |
| GPU history hash | *identical to the above* |
| ticks recorded | 200 (no extinction) |
| final population | 37 |
| births (cumulative) | 30 |
| starved (cumulative) | 1 |
| contention losses (cumulative) | 370 |
| summed IR steps, last tick alone | 1,665 |

Both `cargo test -p cell80-life` (all platforms) and the macOS-only GPU test are green;
`cargo clippy -p cell80-life --all-targets` is clean. Neither test needed a single change
to accommodate the fix — both only assert "two runs/engines agree with each other," never
a fixed golden hash — which is exactly why the fix could land inside `ex0.rs` directly
rather than needing a parallel "fixed" engine.

### The bug this doc originally reported, and what fixing it actually changed

The first version of this section reported **560 organisms** after 200 ticks — wildly
different from `cell80-life`'s original steady ~8–12 under the same grazer genome
(`cell80-life-findings.md` Finding 1) — and named the cause: food-tile eating was
**non-exclusive within a tick** (every organism intending to eat at a tile got the full
snapshot food amount, tile clears once), so organisms converging on a food-rich tile all
got a free, uncontested meal every tick and reproduction compounded instead of being
resource-limited. That was flagged explicitly as something EX-1 would need resolved before
trusting a regime comparison against `cell80-life`'s baseline.

**Fixing it (via `contention.rs`, described above) brings the population down to 37** —
370 contention losses recorded over the run confirms the mechanic engaged constantly, not
as a rare edge case. **37 is much closer to the original ~8–12 band than 560 was, but it
is not an exact match, and that gap is being reported honestly rather than rounded away.**
Plausible remaining causes, untested here: 200 ticks may not be enough for this
population to fully settle (the original binary's steady-state read was itself over a
longer/different run shape); the reproduction-timing rule (decided off post-decay,
pre-move energy, to keep it decision-phase-computable — see `ex0.rs`) still differs
slightly from `cell80-life`'s exact sequential ordering; and a *contested* win still grants
the entire tile's food to one organism (matching the original `eat` cell's semantics
exactly) rather than the original's "whoever the sequential loop reached first" rule,
which is a different selection principle even though both grant a whole meal to exactly
one organism. **This is close enough to treat the fix as working, not close enough to
claim the two engines produce numerically identical population curves** — EX-1, which
actually needs "port the genomes unchanged" to mean something quantitative, should treat
this as the honest starting point, not assume the gap is zero.

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
- **The remaining ~37-vs-~8–12 gap from `cell80-life`'s exact tick semantics**, covered
  above — the contention bug is fixed, but this is not a claim of numerically identical
  population dynamics to the original binary, only "much closer, and the remaining gap is
  named rather than hidden."
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
- Run longer than 200 ticks to see whether the post-fix population (37, still noticeably
  above ~8–12) continues trending down toward the original band or settles at a different
  steady value — 200 ticks wasn't enough to tell which.

## EX-1 — scale the known ecology

**TL;DR: population survival and steadiness scale robustly to 10⁵ organisms, but the
qualitative *regime distinction* between grazer (steady) and rapid_reproducer (boom-bust)
that the 1D CPU baseline showed does not survive the port — in any 2D topology, and, more
specifically, only at small population sizes even in the one topology where it partially
reappears.** That's the honest gate verdict: EX-1 answers its own pre-registered question
("does scale reveal that the small-n stability was a boundary artifact?") with a real
"partly, yes" — not the clean "both regimes reproduce" the gate hoped for, and not the
"populations only survive in a narrow hand-tuned slot" kill condition either. Both gates'
correctness half (replay bit-exact, GPU ≡ interpreter) passed cleanly on the first working
implementation, same as EX-0.

### What was built

- **`contention.rs`** (landed as EX-0's fix, reused here unchanged) — RNG-resolved
  eat-tile contention, shared by both engines.
- **`world2d.rs`** — `World2D`: toroidal, row-major, food placed probabilistically per-tile
  via `rng::draw` (a real, continuous density knob, not a fixed lattice stride).
- **`ex1.rs`** — the 2D tick engine. `sense_move` (the genome's own, completely unmodified
  cell — `argmax3` in both genomes) is called twice per tick, once per axis, combined by a
  host-side rule (act on the axis with the larger sensed food differential, falling back
  to a fixed X-priority only on an exact tie). Never exceeds `rustmsl::IN_STRIDE = 3`, so
  `genes.rs`/`GeneSet` needed zero changes. Also exposes `oscillator_rate`, a direct check
  for organisms getting stuck ping-ponging between tiles (see below).
- **`tests/ex1_cpu_replay.rs`**, **`tests/ex1_gpu_vs_cpu.rs`** — mirror EX-0's two tests for
  the 2D engine; both passed cleanly on the first run, including the two-axis decomposition.
- **`src/bin/ex1_sweep.rs`** — a GPU-only, four-part exploratory/calibration binary (not a
  `#[test]` — this reports a result, per this repo's convention). Reuses `ex1::run` as-is
  (full per-tick history) rather than a separate lightweight-summary path, since part 1
  measures directly whether that retention is actually a problem at the tested scales
  rather than assuming it is.
- **`genes.rs`** gained a small, non-behavioral refactor: `EngineKind`/`batch_run`/
  `sum_steps` moved here from `ex0.rs` so `ex1.rs` doesn't duplicate them — re-verified all
  of EX-0's tests still pass, unchanged, after the move.

### Part 1 — GPU wall-clock at scale (real, not assumed)

| N | world | 20 ticks | ms/tick | final pop |
|---:|---|---:|---:|---:|
| 100 | 25×25 | 36.6 ms | 1.83 | 168 |
| 1,000 | 78×78 | 33.3 ms | 1.66 | 1,641 |
| 10,000 | 245×245 | 71.9 ms | 3.60 | 16,594 |
| 100,000 | 775×775 | 417.5 ms | 20.87 | 161,217 |

Per-tick cost is dominated by the 7 fixed dispatch launches, not by N — it barely moves
across two orders of magnitude and stays comfortably sub-linear even at the top of the
range. **Correction to a number this doc almost borrowed**: docs/14's "flat ~140–180 ms"
megakernel figure is from the *different* `compile_library`/fused-kernel path, not the
plain single-cell `GpuBatch::run` path `genes.rs` actually uses — the numbers above are
measured fresh for this engine, not inherited from that unrelated benchmark. No crash, no
practical memory wall at N=100,000 for a 20-tick smoke run; a full 500–2000-tick run at
that scale would retain a food-array clone per tick (~1.2 MB at 775×775) — hundreds of MB
to a few GB over a long run, a real but not blocking cost, named here rather than hit
unexpectedly later.

### Part 2 — calibration in a true 2D world: the regime split doesn't show up at all

Grazer vs. rapid_reproducer, 1000 ticks, 500-tick tail, 5 seeds, swept across four
world-size/density configs (8×8@0.33, 8×8@0.6, 12×12@0.33, 24×24@0.2):

| config | genome | CV_tail range | R_tail range |
|---|---|---|---|
| every config tested | grazer | 0.025–0.112 | 1.12–1.48 |
| every config tested | rapid_reproducer | 0.034–0.091 | 1.18–1.70 |

The two genomes' numbers overlap almost completely at every config — nothing like the
original CPU baseline's "steady ~8–12 vs. oscillating 2–6, 3× turnover." Populations
survive robustly everywhere (18–205 final organisms, no narrow hand-tuned slot needed) —
it's specifically the *distinction between the genomes* that's missing, not survival
itself.

### Part 3 — isolating the cause: dimensionality, not the axis-decomposition mechanism

Before concluding anything, the mechanism itself needed ruling in or out as the cause. A
height=1 world makes north/south both wrap to "here," so `argmax3`'s own tie-break rule
("ties → lowest index," i.e. "stay") makes the Y axis always report "stay" — the *same*
two-axis code path still runs (still calls `sense_move` twice, still runs the
priority-combination logic), but with only one real movement axis, closely mimicking a 1D
ring:

| ring | genome | CV_tail range | R_tail range |
|---|---|---|---|
| width=24 @ 0.33 | grazer | 0.041–0.090 | 1.27–1.48 |
| width=24 @ 0.33 | rapid_reproducer | 0.081–0.138 | 1.54–2.31 |
| width=8 @ 0.33 | grazer | 0.064–0.128 | 1.31–1.71 |
| width=8 @ 0.33 | rapid_reproducer | 0.113–0.248 | 2.00–4.00 |

**A real separation reappears, and gets sharper as the ring tightens** — rapid_reproducer's
R_tail roughly doubles grazer's at width=8. This rules out the axis-decomposition/
contention mechanism as broken: it *can* reproduce a genuine qualitative split, given a
low-dimensional-enough, tight-enough world. What changed between part 2 and part 3 is
specifically the number of escape routes available (2 directions vs. 4 + diagonal-adjacent
combinations), not anything about how `sense_move`/contention are wired.

### Part 4 — does that split survive at the scale the gate actually asks about? No.

Same height=1 ring, scaled to 1,000 and 10,000 initial organisms (multiple organisms per
tile is unproblematic — this engine has no collision exclusion, matching `ex0.rs`/the
original binary):

| ring | N | genome | CV_tail | R_tail |
|---|---:|---|---:|---:|
| width=100 | 1,000 | grazer | 0.075–0.083 | 1.29–1.33 |
| width=100 | 1,000 | rapid_reproducer | 0.057–0.058 | 1.33–1.35 |
| width=300 | 10,000 | grazer | 0.102–0.108 | 1.39–1.45 |
| width=300 | 10,000 | rapid_reproducer | 0.054–0.055 | 1.32–1.35 |

**The separation found in part 3 collapses again once population scales past a few dozen
— and rapid_reproducer's numbers end up *slightly below* grazer's, if anything, not above.**
That's the real, complete answer to EX-1's pre-registered question: the small-n boom-bust
regime cell80-life originally observed for rapid_reproducer looks like a finite-size/
small-population stochastic effect (individual contested-tile outcomes visibly swinging
population at n≈8–20), not a property of the genome that persists once population is large
enough for aggregate statistics to smooth it out — regardless of whether the world is a
tight 1D-equivalent ring or an open 2D grid.

### Gate verdict

Per doc 17's literal wording — "the two genomes reproduce their qualitative CPU regimes at
scale (grazer steady, rapid_reproducer oscillatory)" — **grazer's half passes** (steady,
low CV/ratio, at every scale and topology tested, 10⁴–10⁵ included) and
**rapid_reproducer's half does not** (no oscillatory regime at any tested scale ≥ ~100
organisms, in any topology). This is not the doc's stated kill condition either
("populations only survive in a narrow hand-tuned slot") — survival is robust everywhere.
The honest characterization: **the regime *distinction* cell80-life observed was itself
scale-fragile, not the ecology's survival.** Report and proceed accordingly — EX-2/EX-3
should not assume rapid_reproducer's original small-n boom-bust characterization transfers
to any larger run without re-checking it the way this section just did.

### Oscillator diagnostic

`oscillator_rate` (period-2 position-cycle fraction, 50-tick window) was **0.000 in every
config tested** — the axis-decomposition movement rule does not measurably get organisms
stuck ping-ponging between tiles, at any scale checked here. The anisotropy (fixed
X-priority on an exact sensed-differential tie) remains a named, minor simplification, not
one shown to produce a behavioral artifact.

### What this does *not* show (deferred, by design)

- Everything EX-0 already deferred (heterogeneous-genome GPU batching, no shared-mutable
  on-GPU world, RNG as host-fn not `.cell`) still applies unchanged to EX-1.
- **Real state-cell GPU batching for a true single-call 5-way movement decision** — every
  arity-4+ stdlib selector (`argmax4`, `argmax4_u32`, `argmin4`, `choose_best4`) is a state
  cell, and `rustmsl`'s batch layout caps a plain-function cell at 3 inputs regardless;
  axis-decomposition was chosen instead (validated above as mechanistically sound, not just
  convenient) and this stays deferred unless a future need can't tolerate the fixed-tie
  anisotropy.
- **A full 500–2000-tick run at true N=10⁵ was not executed** — part 1 confirms the
  per-tick cost and short-run memory footprint are fine; a long run's cumulative
  food-snapshot retention (hundreds of MB–few GB, estimated above) was not actually run to
  completion and measured.
- **The world-size/food-density grid was exploratory (4–6 points, 2–5 seeds), not the full
  8-seed×multi-config grid** the original plan sketched — the mechanistic question (why
  does the split appear/disappear) turned out to matter more than grid coverage, and got
  prioritized over it per the mid-run checkpoint.
- **CV_tail/R_tail as a classifier is doing real work here but wasn't independently
  validated against a third, deliberately-planted "obviously boom-bust" control** (e.g. an
  artificially unstable genome) — the numbers separate grazer from rapid_reproducer in
  part 3 in the expected direction, which is reassuring but not the same as a validated
  classifier.

### Reproduce it

```
cargo test -p cell80-life --test ex1_cpu_replay        # any platform
cargo test -p cell80-life --test ex1_gpu_vs_cpu         # macOS (Metal) only
cargo run -p cell80-life --release --bin ex1_sweep      # macOS (Metal) only, ~1-2 min
cargo clippy -p cell80-life --all-targets
```

### What would raise confidence further

- Run part 4's configs at the seed count part 2/3 used (5, not 2) to confirm the collapse
  at scale isn't itself a coincidence of two particular seeds.
- Actually execute a long (1500+ tick) N=10⁵ run to replace part 1's short-run memory
  estimate with a measured number.
- A deliberately-planted unstable-genome control to validate CV_tail/R_tail as a
  classifier independent of the two genomes this doc has been calibrating against.
- Test whether a food-density *gradient* (not just a uniform probability) changes part 2's
  null result — uniform random placement may itself be diluting the resource clustering
  that drove the original 1D lattice-based food layout's boom-bust dynamic.

## EX-2 — open-ended genome mutation

**TL;DR: both operators are done. Operator (a) (parametric + cell-swap) makes genome
diversity emerge and grow exactly as expected; operator (b) (cell-assembly composition) is
nowhere near "vanishingly rare" — most attempted compositions are viable, and roughly
3-in-10 births in an extended-pool run actively carry a composed gene.** Both gates pass,
and building operator (a) surfaced a real, previously-latent bit-exactness gap in EX-0/
EX-1's own machinery — now fixed. Operator (b)'s own gate — sandbox-safe by construction,
sometimes viable, occasionally fitter, reaching strategies parametric mutation structurally
cannot — is addressed below with real receipts, not assumed.

### Operator (a): parametric + cell-swap

Every organism now carries its own genome (three numeric fields + three swappable-role
pool indices); mutation on reproduction produces measurable, growing diversity
(dispatch-count-per-role climbs from 1 to 7–30 over 2,000 ticks); 95.7% of births carry at
least one role differing from the run's starting genome. Replay is bit-exact and GPU
agrees byte-for-byte with the CPU-reference interpreter, including the new
grouped-by-pool-index dispatch.

### What was built (operator a)

- **`pools.rs`** — role-pool discovery lifted from `main.rs`'s original `discover_pools`
  (untouched): 85 promoter candidates, 43 movement candidates discovered from the current
  library (up from `main.rs`'s own comment citing 54/26 at an earlier library size — the
  pools have grown with the stdlib since).
- **`rng.rs`** — 12 new streams, one per independent mutation decision
  (`MUTATE_{DECAY,THRESHOLD,GIVE_PCT}_{CHANCE,MAGNITUDE}_STREAM`,
  `MUTATE_{HUNGRY,REPRO,SENSE}_SWAP_{CHANCE,TARGET}_STREAM`), plus `chance()` and
  `pick_other_index()` — a pure, index-exclusion reimplementation of `main.rs`'s stateful
  `pick_other` rejection loop, unit-tested for uniform coverage and order-independence.
- **`genes.rs`** — three additive changes: `CompiledGene.gpu` is now `Option<GpuBatch>`
  (`None` for a composed candidate — see operator (b) below); `CompiledGene::from_funcs`
  (builds a `CompiledGene` directly from already-lowered IR, used by operator (b)'s
  composed candidates); `batch_run_grouped`, the heterogeneous-cell-choice dispatcher (one
  GPU call per distinct pool index in use, not per organism), unit-tested against a naive
  per-organism loop.
- **`ex2.rs`** — the tick engine: `GenePools` (fixed `decay`/`eat`/`split` + swappable
  `hungry_pool`/`repro_pool`/`sense_pool`), per-organism `OrgGenome`, and `mutate()`
  applying operator (a)'s two mutation kinds via the 12 new streams. Movement/contention/
  world are otherwise identical to `ex1.rs` — only genome *content* became heterogeneous.
- **`history.rs`** — additive `OrgSnapshot2DGenome`/`TickRecord2DGenome`/
  `absorb2d_genome`/`BirthEvent` (a light per-birth log: child/parent id, tick, post-
  mutation genome — reusable by EX-4's lineage instrumentation later, not duplicated
  there).
- **`tests/ex2_cpu_replay.rs`**, **`tests/ex2_gpu_vs_cpu.rs`** — mirror EX-0/EX-1's two
  tests, plus an explicit assertion that mutation actually produced observable drift (a
  replay gate on a run where nothing ever mutated would prove nothing about EX-2 specifically).
- **`src/bin/ex2_mutation_report.rs`** — genome-diversity-over-ticks and dispatch-count
  receipts (not a `#[test]`, matching `ex1_sweep.rs`'s reporting convention).

### A real bug this pass surfaced (and fixed) in EX-0/EX-1's own machinery

The first real test run panicked: `interp run 'unit_div': interp: halt(65286)`. Cause:
`genes.rs::run_cpu` treated *any* interpreter error as a bug and panicked — a choice that
was never actually safe, just never exercised, because EX-0/EX-1's six curated gene cells
(`sub_sat`/`is_gt`/`add_sat`/`argmax3`/`is_ge`/`discount_percent`) happen not to trap under
the numeric ranges those experiments' ticks produce. EX-2's cell-swap pool calls *arbitrary*
same-signature stdlib cells — not curated for this use — with arbitrary organism-supplied
inputs, and some of them (like `unit_div`, a guarded division cell) legitimately halt on
certain inputs. That's not a crash-worthy defect; it's a normal, well-typed trap the GPU
kernel already handles by encoding it in the output sextet's `status`/`r0` fields, never by
erroring the whole dispatch.

**Fixed by reusing the exact, already-proven fold `cell80/tests/msl_battery.rs`'s
`interp_quad` established**: divide-by-zero and fuel-exhaustion fold to `r0 = 0`;
`halt(code)` folds to `r0 = code` — parsed from the same `"interp: halt(N)"` error-string
convention `msl_battery.rs` already parses, not a new one invented here. Any *other* error
still panics (a real defect, not a trap). Re-verified `cargo test -p cell80-life` fully
green afterward, including EX-0/EX-1's existing tests (their six curated cells never
exercise this path, so their behavior is unchanged) — the fix is strictly additive
robustness, not a behavior change for anything already shipped.

**Why this was a latent gap, not a new one**: `run_gpu_batch` never had this problem — a
GPU thread's halt/trap was always encoded in its output sextet, never propagated as a
dispatch-level error. The gap was specifically that `run_cpu` and `run_gpu_batch` disagreed
on how to represent the *same* trap event, which would have broken "GPU ≡ interpreter"
bit-exactness the moment any exercised cell actually trapped — EX-0/EX-1's curated cells
just never did. This is exactly the kind of gap the project's own diff-battery discipline
(`msl_battery.rs`) exists to catch at library scale; EX-2 caught the ecology-specific
instance of it empirically, the first time this codebase's execution path was pointed at
truly uncurated cells with adversarial (mutation-selected) inputs.

### Receipts

Run: grazer starting genome, seed `0x5eed_1234_c311_80ff`, 8 initial organisms, 32×32
toroidal world, density 0.2, 2,000 ticks, GPU engine.

| tick | n | dispatch: hungry / repro / sense | avg decay / thresh / give% |
|---:|---:|---|---|
| 0 | 8 | 1 / 1 / 1 | 1.0 / 200 / 50% |
| 200 | 110 | 3 / 4 / 8 | 1.1 / 202 / 49% |
| 600 | 165 | 6 / 18 / 8 | 1.1 / 203 / 50% |
| 1,000 | 152 | 7 / 21 / 10 | 1.0 / 203 / 50% |
| 1,400 | 195 | 11 / 25 / 14 | 1.1 / 201 / 48% |
| 1,800 | 216 | 7 / 30 / 13 | 1.1 / 201 / 47% |

- 2,000 ticks in 22.9 s (11.4 ms/tick) at this population/dispatch-count scale — consistent
  with the ~0.2 ms/dispatch micro-benchmark (up to ~30 repro-role dispatches alone at peak
  diversity, plus hungry/sense, roughly accounts for the observed per-tick cost).
- 11,200 total births; **95.7% carry at least one role differing from the run's starting
  genome** — mutation is not a rare event here, it's the dominant outcome per birth.
- Numeric fields stay in a tight band around their starting values (decay≈1.0–1.2,
  threshold≈200–203, give≈47–50%) even after thousands of mutation events — consistent
  with `cell80-life-findings.md`'s Finding 3 (stabilizing selection, not neutral drift),
  now reproduced in the GPU-batchable 2D engine independently.
- `cargo test -p cell80-life` (both platforms) and `cargo clippy -p cell80-life
  --all-targets` are green.

### What this shows

- Operator (a) — parametric + cell-swap — ported cleanly into the GPU-batchable engine.
  Heterogeneous numeric parameters needed zero new dispatch machinery (already free);
  heterogeneous cell choice needed exactly one new mechanism (`batch_run_grouped`), and
  the micro-benchmark-predicted "grouping stays cheap" held up in the full engine, not
  just the isolated benchmark.
- The replay/GPU-parity gates hold under real, active mutation-driven diversity — a
  meaningfully harder test than EX-0/EX-1's single fixed genome, since it now exercises
  dozens of distinct pool members' actual trap/non-trap behavior across two bodies, not
  just six curated, never-observed-to-trap cells.

### What this does *not* show (deferred, by design)

- **No fitness signal beyond survival/reproduction was measured for specific swapped-in
  cells** — this pass reports that diversity grows and stabilizes, not which particular
  pool members are over- or under-represented relative to a null-drift expectation (that
  would need the population-genetics-style analysis `cell80-life-findings.md` flagged as
  future work for its own Finding 3/4, still open here too).
- **Everything EX-0/EX-1 already deferred still applies** (heterogeneous-*genome*-shape
  GPU batching beyond pool-index grouping if diversity ever approaches population size;
  no shared-mutable on-GPU world; RNG as host-fn not `.cell`).
- **The trap-folding fix changes what "sandbox-safe" means operationally, worth stating
  precisely**: a halting/trapping pool member is now a defined, non-fatal outcome (reads as
  the halt code or 0), not a crash — but this is a *representation* fix (CPU now agrees
  with what GPU already did), not a new safety *guarantee* about which cells are sensible
  to use as a promoter/movement gate. A halting cell can still be swapped in and simply
  "mostly reads as false/stay," exactly as `main.rs`'s own doc comment already described
  for non-boolean/out-of-range candidates — this pass extends that same tolerance to
  candidates that trap outright, rather than special-casing them.

### What would raise further confidence in operator (a) specifically

- A population-genetics-style analysis of which specific pool members become
  over-represented vs. a neutral-drift null, mirroring `cell80-life-findings.md`'s own
  flagged-but-undone control (mutation-disabled-after-founding) for its Finding 3/4.
- Confirm the dispatch-count-stays-cheap finding at a larger population scale than this
  pass's ~150–250 (EX-1's 10⁴–10⁵ organism runs, with mutation now turned on).

### Operator (b): cell-assembly composition

The design doc's actual hypothesis: can mutation act on the bytecode itself — not just
numbers and swaps among *existing* cells — producing organisms whose behaviour is novel and
still sandbox-safe? Shipped as arity-preserving 2-cell wiring
(`run(a0..aN) = g(a0,..,f(a0..aN),..,aN)`, f's output replacing one of g's argument slots),
the design doc's own pre-registered kill/rescope fallback — reusing the exact
`Expr::Call` + `linearize`-does-the-inlining trick a concurrent session's
`cell80/examples/gpu_grow.rs` already proved at population scale, generalized here from
unary chains to arity-preserving wiring since none of EX-2's three swappable roles (2- or
3-arg) are unary.

### What was built (operator b)

- **`composition.rs`** — `ComposablePool::discover` (cells whose lowered form is exactly
  one self-contained function *and* carries no const data — narrower than the swap pool,
  and a real, checked rejection, not an assumed non-issue: an earlier draft of this file
  silently discarded const data instead of checking for it, a correctness bug caught before
  it shipped); `generate_and_gate` (structural bound via `rustmsl::interp::linearize`,
  viability via `rustmsl::interp::cpu_run` over `cell80::DEFAULT_PROBES` with `Err` handled
  as a counted stillbirth rather than panicked, novelty via
  `Fingerprint::from_value_sextets` against every existing pool member — reusing
  `admission.rs`'s `DUPLICATE_AGREEMENT` threshold directly, never calling `admission::admit`
  itself, which would wrongly refuse every candidate for having no retrieval rows); **a
  cross-interpreter consistency check** beyond what was originally planned — every viable
  candidate is re-run through `cell80_core::Interp` (the actual body
  `CompiledGene::run_cpu` executes it through once admitted) and rejected as not-viable on
  any disagreement with `rustmsl`'s bytecode VM, applying this project's own
  "a disagreeing executor is a defect, never expected variance" discipline to a second CPU
  interpreter, not just the GPU body; `grow_pool` (a deterministic, seed-driven offline
  sweep — mirroring `main.rs`'s own one-time-at-startup `discover_pools`, not a live
  per-tick event).
- **`Cargo.toml`** — `rustmsl` promoted from macOS-gated to an unconditional dependency:
  `rustmsl::interp`'s `linearize`/`cpu_run` are not macOS-gated inside `rustmsl` itself
  (only its `GpuBatch`/`InterpBatch` modules, which depend on the `metal` crate, are) —
  composition's structural/viability gates build and run on every platform.
- **`ex2_mutation_report.rs`** extended with a composition sweep (300 attempts/pool) and an
  ecology adoption experiment: build a "control" `GenePools` (disk-loaded movement pool
  only) and an "extended" one (the same pool plus every viable composed candidate,
  compiled via `CompiledGene::from_funcs` — CPU-only, `gpu: None`, even though the run
  otherwise uses the GPU engine; composed cells are rare enough that this doesn't cost
  anything the receipts below show as significant), then run both and report adoption.

### Receipts

Same grazer/32×32/density-0.2/2000-tick/GPU config as operator (a)'s report, seed
`0x5eed_c0de_c0de_5eed` for the composition sweep:

| | promoters (arity 2) | movement (arity 3) |
|---|---:|---:|
| composable (single-func, no consts) | 80/85 | 34/43 |
| fingerprinted for novelty | 75/85 | 42/43 |
| attempts | 300 | 300 |
| structurally invalid | 43 | 0 |
| not viable (traps or cross-interpreter mismatch) | 75 | 0 |
| duplicate (agreement ≥ 1.0 vs. an existing cell) | 9 | 50 |
| **viable** | **173 (57.7%)** | **250 (83.3%)** |
| closest-existing-match agreement, viable candidates | avg 0.590, max 0.950 | avg 0.696, max 0.950 |

Ecology adoption, extending the movement pool from 43 to 293 (43 + 250 viable composed
candidates), same seed as every other report in this doc:

| | control | extended |
|---|---:|---:|
| final population | 253 | 324 |
| total births | 11,200 | 21,413 |
| births carrying a composed `sense_move` gene | — | 6,344 / 21,413 (29.6%) |
| avg direct children — composed-gene carriers | — | 0.829 |
| avg direct children — disk-gene carriers | — | 1.067 |

`cargo test -p cell80-life` (both platforms) and `cargo clippy -p cell80-life
--all-targets` are green, including the 4 new composition unit tests (composable-pool
discovery, an end-to-end gate-outcome sweep, a deliberately-trapping pair proving
stillbirth-not-crash, and `grow_pool` determinism).

### What this shows

- **Viable bytecode mutations are not vanishingly rare — the opposite.** 57.7%/83.3%
  viable directly contradicts the design doc's own worried-about failure mode ("if viable
  bytecode mutations are vanishingly rare... fall back"). The kill/rescope condition simply
  didn't fire; this is a real, positive result for operator (b), not a fallback.
- **Composed candidates are behaviourally novel, not near-duplicates dressed up as new** —
  closest-match agreement averaging 0.59–0.70 (well below the 1.0 duplicate bar, in the
  same range `evolved-cells-findings.md` reported for its own genuinely-novel discoveries)
  shows real, if related-to-something-existing, behavior.
- **The ecology actually exploits them, substantially** — 29.6% of all births in the
  extended run carry a composed gene. This is the direct, real-data answer to "does
  bytecode mutation reach strategies parametric mutation cannot": the swap-only pool
  structurally cannot ever produce these genomes (they don't exist as named cells), and
  nearly a third of births in the extended run carry one anyway.
- **"Occasionally fitter" is not shown by the aggregate, and that's reported honestly, not
  smoothed over.** Composed-gene carriers averaged *fewer* direct children than disk-gene
  carriers (0.829 vs. 1.067) in this run — adoption is real, but the aggregate doesn't show
  composed genes as more fit on average. This doesn't rule out individual composed
  candidates being fitter than their specific parent (the doc's literal criterion) — but a
  per-candidate counterfactual (see *Follow-up* below) now tests exactly that and finds
  **none** in a 15-origin sample, so the aggregate isn't concealing a fitter minority here.
- **A real correctness bug was caught before shipping, by the same discipline that caught
  EX-2 operator (a)'s trap-folding gap**: the first draft of `ComposablePool::discover`
  discarded a cell's const data rather than checking it was empty — harmless only because
  none of the actually-composed cells in this sweep happened to need any, caught by
  re-reading the code against what `rustz80::lower_program_full`'s `Lowered::const_data()`
  actually returns, not by a failing test.

### What this does *not* show

- **The between-run (control vs. extended) comparison has a stated confound**: a larger
  pool changes which index every swap draw lands on from the first mutation event onward,
  so the two runs' populations diverge immediately — the 253-vs-324 final population
  difference is not a clean causal read of "composed candidates grow the population," and
  isn't presented as one. The within-run comparison is the primary signal.
- **Only the movement pool (arity 3, `sense_move`) was extended for the adoption
  experiment** — the promoter pool (shared by `hungry_promoter`/`repro_promoter`) would
  need a more careful design to extend one role without silently affecting the other, since
  `ex2.rs`'s `GenePools` compiles `hungry_pool`/`repro_pool` as separate instances from the
  same name list; not attempted this pass.
- **Composed candidates execute CPU-only even in the "GPU" engine run** — a stated v1
  restriction (see the design doc), not yet a demonstrated GPU-compiled composed cell.
- **Pool members carrying const data are excluded from both composability and the novelty
  comparison** (75/85, 42/43 fingerprinted, not the full pools) — a narrow, stated gap: a
  composed candidate could in principle duplicate a const-bearing cell's behavior without
  the novelty check catching it.
- **"Occasionally fitter than a specific parent"** — now measured by counterfactual replay
  (*Follow-up* below), not left to the aggregate: 0/15 focal composed genes beat their
  specific parent. What remains open is scale (15 origins on one seed), not the direction.

### Follow-up: the per-candidate counterfactual (EX-4 machinery, resolving "occasionally fitter than parent")

The aggregate above is *observational* — composed-gene carriers and disk-gene carriers are
different organisms in different micro-contexts, so 0.829 vs. 1.067 is a correlation, not a
causal read of any one gene. `ex4_fitness_counterfactual.rs` closes that with the exact
counterfactual `ex2::run_with_overrides` was built for: take one organism that acquired a
composed `sense_move` gene by mutation from a *disk-gene parent*, revert exactly that one
swap so the *same* organism (same id, birth tick, position, energy — every tick before its
birth is byte-identical between the two runs) instead inherits its parent's disk gene, replay
the identical world, and compare that organism's own offspring with vs. without the composed
gene. Direct children is the primary metric (the same one the aggregate used); transitive
descendant count is a more sensitive secondary that also catches divergence the fork rippled
downstream.

Same extended run as above (seed `0x5eed_1234_c311_80ff`, 293-gene movement pool, 21,413
births), 15 focal births sampled evenly across the 1,157 disk-parent→composed-child origins,
one full GPU replay each (~25 s/replay, 384 s total):

| Δ (composed − parent's disk gene) | mean | fitter | equal | worse |
|---|---:|---:|---:|---:|
| direct children | −0.600 | 0 | 12 | 3 |
| transitive descendants | −5.800 | 0 | 11 | 4 |

**Not one composed gene beat its specific parent, on either metric.** The 12/15 neutral on
direct children are organisms that would have had ≈0–1 children either way (late,
resource-limited, or short-lived); every organism actually *positioned to reproduce* lost by
switching to the composed gene — org 9 (a tick-5 founder-child) went from 2 children / 9
descendants on its parent's disk gene to 0/0 with a composed gene; org 7216 from 7/39 to 1/1;
org 13920 was neutral on direct children (3 vs. 3) but shed 39 descendants (52 → 13). This is
*uniform* purifying selection, not merely an unfavourable average, and the answer to the
design doc's literal "occasionally fitter than a specific parent" criterion is, in this world
and this sample, **no**.

Two things this settles, both pointing the same way as the aggregate rather than against it:

- **The counterfactual is *more* negative than the observational gap** (−0.600 vs. −0.238
  direct children) because it isolates the causal switch away from the incumbent-fit disk
  gene (every focal parent carried disk gene 1, the ancestral winner) instead of pooling
  composed carriers that inherited the gene neutrally deep in a lineage. Conditioning on the
  moment the gene actually enters makes its cost clearer, not weaker.
- **The 29.6% adoption headline is a below-neutral standing frequency, not evidence of
  exploitation.** A simple neutral-drift argument: the swap fires at 8 %/birth (`SWAP_MUTATE_PCT`)
  and resamples `sense_move` uniformly over the pool, which is 250/293 ≈ 85 % composed, so
  under *no* fitness difference the stationary composed fraction among births would sit near
  85 %. Observed 29.6 % is far below that — mutation keeps injecting composed genes and
  selection keeps pruning them back down. "Reachable by bytecode mutation, and structurally
  impossible for the swap-only pool" stays true and is the real operator-(b) result; the
  stronger reading "the ecology exploits them" does not survive the baseline.

Caveats, in the usual spirit: n = 15 on a single seed, and only ~3–4 focal organisms were
fecund enough to be individually informative, so the *direction* is unambiguous (0/15
positive; every informative case negative) but a wider sweep would tighten the magnitude. The
metric is in-context lineage fitness — it includes the ecological ripple the reverted
organism's changed movement causes — which is the honest counterfactual but makes "fitter"
world-conditional, not a context-free gene property.

### Reproduce it

```
cargo test -p cell80-life --test ex2_cpu_replay              # any platform
cargo test -p cell80-life --test ex2_gpu_vs_cpu               # macOS (Metal) only
cargo test -p cell80-life --lib composition                   # any platform
cargo run -p cell80-life --release --bin ex2_mutation_report  # macOS (Metal) only, ~2 min
cargo run -p cell80-life --release --bin ex4_fitness_counterfactual  # macOS (Metal) only, ~6 min
cargo clippy -p cell80-life --all-targets
```

### What would raise confidence further

- Extend the promoter pool too (both roles, or one at a time with a deliberate
  single-role-extension mechanism), not just movement.
- A wider per-candidate counterfactual sweep — the *Follow-up* above ran 15 origins on one
  seed and found 0 fitter; more origins across more seeds would tighten the magnitude (the
  direction is already unambiguous, and the machinery is now in
  `ex4_fitness_counterfactual.rs`).
- Include const-bearing pool members in the novelty comparison via a fingerprinting path
  that actually plants their const data, closing the stated gap.
- A larger composition sweep (the doc's own kill/rescope condition didn't fire at
  300 attempts/pool, but a wider sweep would tighten the viable-fraction estimate).

## EX-3 — predator/prey co-evolution

**TL;DR: the two-species engine itself works cleanly (bit-exact replay, GPU ≡
CPU-reference, including predation-kill contention), and the pre-registered mutation-off
control landed a genuinely strong result — mutation is causally necessary for predator/prey
coexistence at this population scale, replicated 10/10 seeds across two independently-robust
world configs, and *not* explained by an overhunting-mechanic defect (a satiation cooldown,
built specifically to rule that out, doesn't rescue the mutation-off case either, also
10/10). But the flagship claim — a traceable, coupled co-evolutionary arms race between the
two species — was not found.** Under a rigorous permutation-null significance test (not just
an eyeballed "the events look like they alternate"), every one of 6 long (10,000-tick) seeds
came back statistically indistinguishable from chance interleaving of two independently
noisy evolving populations (p = 0.13–0.99). Per the user's explicitly chosen bar (temporal
pattern **and** counterfactual confirmation, the stricter of the two options offered), the
claim fails at the first prong, so no counterfactual replay was run — doing so on a
non-significant event would misrepresent what the data shows. Reported honestly as the
design doc's own stated anti-artifact kill condition: real population dynamics and a real
mutation-dependence result, but not demonstrated co-evolution.

### What was built

- **`history.rs`** — `Species` (heritable, never mutated — sits *beside* `OrgGenome`, not
  inside it), `OrgSnapshot2DEco`/`TickRecord2DEco` (+ `predation_kills`, the receipt analogue
  of `contention_losses`)/`BirthEventEco`, `HistoryHasher::absorb2d_eco`.
- **`contention.rs`** — additive `PREDATION_CONTENTION_STREAM`; `resolve_eat_contention`
  refactored into a thin wrapper over a new generalized `resolve_contention(seed, tick,
  candidates, stream)`, since predation-kill contention needed its own stream and the old
  function had `EAT_CONTENTION_STREAM` hardcoded inside its body.
- **`predation.rs`** — `PreyIndex`: an O(n)-build/O(1)-lookup position-keyed index built once
  per tick from the tick-start snapshot, replacing `main.rs`'s original `prey_at` (an O(n)
  linear scan *per lookup*, ~5 lookups/predator/tick in 2D) — at EX-1's demonstrated
  10⁴–10⁵-organism scale that would cost O(n_predators × n_total) per tick, plausibly
  dominating everything else by orders of magnitude. Gives co-located grazers an explicit
  lowest-id tie-break, a deliberate choice replacing `main.rs`'s accidental
  `Vec`-scan-order one.
- **`ex3.rs`** — the tick engine. `Org` carries `species` beside `genome` (never mutated,
  copied verbatim parent→child); grazer sensing is unchanged from `ex2.rs` (food-driven);
  predator sensing goes through `PreyIndex`, with `EXPLORE_BIAS` generalized from `main.rs`'s
  1D left/right nudge to a 4-direction cycle (still a pure function of `tick`, no RNG) so a
  predator with zero sensed prey doesn't freeze. Reuses `ex2::GenePools`/`ex2::mutate`
  (bumped to `pub(crate)`) directly — no per-species duplication. Predation-kill resolution
  is a new host-side stage (`killed_victims_from`, independently unit-tested): **a kill
  overrides everything else that tick for the victim** — no eat, no reproduction, simply
  removed — a deliberate clarification of `main.rs`'s own sequential-`Vec`-order accident,
  which has no batched-engine equivalent. `run`/`run_with_overrides` mirror EX-4's exact
  split (a thin `run_impl` wrapper), needed for a future counterfactual replay. A
  `predator_satiation_ticks` cooldown (0 disables it) was added per an explicit Checkpoint B
  decision — ecological state, not a genome field, matching `world2d.rs`'s `regrow_at` idiom.
- **`lineage.rs`** — additive: `LineageTree::build_from_genesis_ids` (a second species'
  genesis ids don't start at 0; `build` is now a thin wrapper), `GenomeFields::from_birth_eco`/
  `from_snapshot_eco`, and `eco_ticks_to_genome`/`eco_births_to_genome` — a
  species-filtering adapter that converts one species' view of a two-species run into the
  *exact* shapes `detect_plurality_events`/`find_origins` already consume, so those functions
  run **completely unmodified** on a second species. No `_eco` variant of either was needed.
- **`tests/ex3_cpu_replay.rs`**, **`tests/ex3_gpu_vs_cpu.rs`** — mirror EX-2's two gates,
  plus explicit assertions that predation actually engaged (kills > 0) and both species
  survived to the end (not a one-sided collapse). 3 hand-constructed ordering-rule unit
  tests (`ex3::ordering_tests`) pin down the "kill overrides eat/reproduce regardless of the
  victim's own eat-contention outcome" rule specifically. 4 new `lineage.rs` unit tests cover
  the species-filtering adapter and the non-zero-genesis-range case.
- **`src/bin/ex3_predator_prey_report.rs`** — Part 1: world-size/ratio/density calibration
  sweep (per-species tail CV/ratio, mirroring `ex1_sweep.rs`). Part 2: the pre-registered
  mutation-off control. Part 3: the satiation-mechanic sweep.
- **`src/bin/ex3_arms_race_report.rs`** — the coupled-trait-change detection: a long flagship
  run per seed, both species' plurality-event timelines via the species-filtering adapter,
  and a permutation-null significance test on the observed cross-species alternation.

### Receipts — Checkpoint A (sanity)

A basic config (48×48 world, 40 grazers, 8 predators, density 0.25, 400 ticks, no satiation
yet) showed predation genuinely engaging — 539 kills, ending at 471 grazers / 149 predators,
not a one-sided collapse — before further investment, with both correctness gates green.

### Receipts — Part 1: calibration sweep (5 seeds, 3,000 ticks each)

| world | ratio | density | outcome (5 seeds) |
|---|---|---|---|
| 32×32 | 30:6 | 0.25 | unstable — 2/5 full extinction, 1/5 predator-only extinction, 2/5 coexist |
| 48×48 | 40:8 | 0.25 | mostly stable — 4/5 coexist, 1/5 predator extinction (grazers unchecked to 1,598) |
| 48×48 | 60:10 | 0.30 | **fully robust — 5/5 coexist** (grazers 1,159–1,528; predators 306–497; 62k–83k kills) |
| 64×64 | 60:10 | 0.25 | **fully robust — 5/5 coexist** (grazers 1,087–1,980; predators 473–561; 33k–52k kills) |

### Receipts — Part 2: the mutation-off control (both robust configs, 5 seeds each, 3,000 ticks)

| | mutation ON | mutation OFF |
|---|---|---|
| 48×48 @ 60:10/0.30 | 5/5 coexist (grazers 1,159–1,528; predators 306–497) | **5/5 predator extinction** (grazers crash to 19–193, one seed fully extinct) |
| 64×64 @ 60:10/0.25 | 5/5 coexist (grazers 1,087–1,980; predators 473–561) | **5/5 predator extinction** (grazers crash to 68–229) |

10/10 mutation-off seeds collapse to predator extinction at the exact two configs that are
otherwise fully robust with mutation on — ruling out "the config just needed more room" as a
confound, since this was deliberately re-tested at the *strongest* configs, not just the
original borderline one.

### Receipts — Part 3: the satiation mechanic (Checkpoint B)

Built per an explicit decision (not skipped, despite the mechanic itself already looking
innocent — see "What this shows" below) to rule out an overhunting confound directly.
`predator_satiation_ticks = 20`, same 2 configs × 2 mutation states × 5 seeds:

| | mutation ON (satiation=20) | mutation OFF (satiation=20) |
|---|---|---|
| 48×48 @ 60:10/0.30 | 5/5 coexist, *healthier* (grazers 1,545–2,275, higher than satiation=0) | **5/5 predator extinction still** (grazers 15–52) |
| 64×64 @ 60:10/0.25 | 5/5 coexist (grazers 1,879–3,765) | **5/5 predator extinction still** (grazers 56–282) |

### Receipts — coupled-trait-change detection (6 seeds, 10,000 ticks each)

Flagship config: 48×48, 60 grazers : 10 predators, density 0.30, `predator_satiation_ticks =
20`, mutation on. All 6 seeds sustained robust coexistence (grazers ~1,700–2,400, predators
~430–550). Both species' plurality-event timelines (`Role::Hungry`/`Repro`/`Sense` ×
`Species::Grazer`/`Predator`, K=5, sampled every 20 ticks) were dense — 41–56 events per
10,000-tick seed — with vote-shares consistently in the 0.2–0.5 range (near-ties, never a
majority), and a longest cross-species-alternating chronological run computed per seed:

| seed | events | longest alternating run | P(run this long by chance, 20,000 shuffles) |
|---:|---:|---:|---:|
| `0x2a` (42) | 41 | 6 | 0.3448 |
| `0x1` | — | 5 | 0.9110 |
| `0x2` | 56 | 4 | 0.9897 |
| `0x3` | — | 6 | 0.7127 |
| `0x3e7` (999) | — | **9** | **0.1338** |
| `0x5eed1234c31180ff` | — | 6 | 0.6916 |

Not one seed reaches conventional significance — even the longest observed run (9, seed 999)
is matched or exceeded by pure random relabeling 13% of the time.

### What this shows

- The species-branching engine design holds up exactly as planned: `Species` beside
  `OrgGenome` (not inside it), one shared `GenePools`/`mutate()`, `PreyIndex`'s O(1) lookup,
  and the generalized `resolve_contention` all compose correctly and deterministically —
  both gates (bit-exact replay, GPU ≡ CPU-reference) passed on the same run that produced
  every number above, including the satiation mechanic and 10,000-tick flagship runs.
- **Mutation is causally necessary for predator/prey coexistence at this scale.** This is a
  strong, well-powered result (10/10 mutation-off seeds collapse, at two configs
  independently validated as fully robust with mutation on) — directly answering the
  pre-registered control (design decision 8).
- **The satiation mechanic's null result is itself informative, not wasted effort.** The
  *same* satiation-less predation mechanic sustains 10/10-seed healthy coexistence with
  mutation on; adding a kill-cooldown doesn't rescue the mutation-off case *at all* (still
  10/10 predator extinction) and, if anything, makes the mutation-on case healthier (larger
  grazer populations). This rules out "the mechanic has a satiation-shaped hole" as the
  explanation for the mutation-off collapse — it's the fixed, non-adapting genesis genome
  that fails, not the mechanic. Building it and testing empirically (per explicit decision)
  gives this a firmer footing than arguing it away would have.
- **The species-filtering adapter design paid off exactly as intended**: `detect_plurality_events`/
  `find_origins` needed zero new code to run per-species on a two-species run — only
  `eco_ticks_to_genome`/`eco_births_to_genome` (data reshaping) and
  `build_from_genesis_ids` (a non-zero id range) were new.
- **A rigorous null model matters at this population scale, not just an eyeballed
  impression.** A "9-event alternating run" sounds long; the permutation test shows it's
  unremarkable (p=0.13). Every seed's raw run length would have read as suggestive evidence
  without this check — exactly the anti-artifact discipline the design doc's own framing
  ("population oscillation is ecology; traceable coupled trait change is co-evolution, and
  only the latter counts") was written to guard against.

### What this does *not* show

- **No coupled trait-change / co-evolutionary arms race was found.** Under the user's chosen
  bar (temporal pattern *and* counterfactual confirmation — the stricter of the two options
  offered), the claim fails at the first prong in all 6 seeds tested, so the counterfactual
  half (`ex3::run_with_overrides`, already built and verified working) was never exercised
  on this claim — running it on a non-significant event would manufacture a false
  confirmation.
- **A real structural limitation, not just a null result: grazers have no predator-sensing
  channel at all in this model.** `sense_move`/`hungry_promoter` are always food-driven for
  grazers — never informed by predator presence, distance, or behavior. Any true "prey
  response" to a predator adaptation could only ever act through differential mortality
  (whichever grazer genome/position/timing happens to survive predation better propagates
  more), a far weaker and more diffuse coupling channel than a direct sense-and-react
  adaptation. This may be a genuine reason no signal was found — the channel for coupling
  was always going to be faint even if real.
- **Only 6 seeds × 10,000 ticks were tested.** A larger seed sweep or substantially longer
  runs might surface a genuine signal this pass didn't reach, or might further confirm the
  null — this is not an exhaustive search, matching EX-4's own stated precedent of
  demonstrating on the seeds/events actually found rather than surveying broadly.
- **The permutation test's null model is a reasonable standard baseline, not the only
  possible one** — uniform random relabeling of the same tick positions; it doesn't preserve
  each individual stream's own event-rate clustering structure, which a stricter check might.
- Numeric-field ("continuous-trait") arms race and predator-vs-predator interaction stay out
  of scope, the same stated limitations `lineage.rs`/`main.rs` already carry forward.

### Reproduce it

```
cargo test -p cell80-life --lib ex3                                # any platform (ordering-rule unit tests)
cargo test -p cell80-life --lib lineage                            # any platform (species-filtering adapter tests)
cargo test -p cell80-life --lib predation                          # any platform (PreyIndex tie-break tests)
cargo test -p cell80-life --test ex3_cpu_replay                    # any platform
cargo test -p cell80-life --test ex3_gpu_vs_cpu                    # macOS (Metal) only
cargo run -p cell80-life --release --bin ex3_predator_prey_report  # macOS only, ~1 hour (full sweep)
cargo run -p cell80-life --release --bin ex3_arms_race_report      # macOS only, ~20 min (6 seeds x 10,000 ticks)
cargo clippy -p cell80-life --all-targets
```

### What would raise confidence further

- Give grazers an actual predator-awareness sensing channel (a `PreyIndex`-style reverse
  lookup reporting predator proximity), so a genuine sense-and-react coupling channel exists
  to detect at all, rather than relying solely on the much fainter differential-mortality
  channel this pass was limited to.
- A longer flagship run (50,000+ ticks) or a larger seed sweep, to rule out that 10,000
  ticks/6 seeds is simply too short/narrow a window for a real signal to separate from noise.
- A stricter or alternative null model for the permutation test (e.g. one that preserves
  each event stream's own clustering/rate structure rather than uniform relabeling).
- If a future pass does find a statistically significant temporal pattern, complete the
  second prong: revert the traced origin via `ex3::run_with_overrides` (already built,
  already exercised by the correctness gates) and confirm causation, exactly as EX-4 did for
  the single-species case.

## EX-4 — the lineage record

**TL;DR: exactly the research artifact the design doc promised, and it worked cleanly on
the first real run that found a mutation-bearing event.** Every genome is now
content-addressed (SHA-256 over its 6 heritable fields); a lineage tree built from EX-2's
existing birth log traces any living organism's genome back to either a genesis organism or
the specific birth that mutated it in; a detector finds real, sustained plurality shifts in
which pool member the population favors for a role; and — the actual gate — reverting
*exactly* the traced mutation and replaying removed the detected shift, while every tick
before the reverted birth stayed byte-identical to the original run. "Evolution you can
single-step and diff" is not aspirational here; it ran.

### What was built

- **`lineage.rs`** — `GenomeFields` (the 6 shared heritable fields, hashed once rather than
  duplicating byte-layout logic across `BirthEvent`/`OrgSnapshot2DGenome`/
  `StartingGenome2`); `LineageTree` (keyed on `(genome_hash, child_id)`, not hash alone, so
  two organisms that independently mutate to the *same* genome stay distinct, queryable
  nodes rather than being silently merged into one with an ambiguous parent);
  `detect_plurality_events` (a `BTreeMap`-based, lowest-index-wins-ties plurality tracker,
  sampled every 20 ticks — matching `cell80-life-findings.md`'s own hand-analysis cadence —
  reporting a "sustained plurality change" only once the new winner holds for `K` further
  samples, not a single-sample blip); `find_origins` (a backward ancestry walk from the
  organisms *actually alive and carrying the winning value* at the event tick, not a
  forward scan that could misattribute an extinct branch). 7 synthetic unit tests
  (hand-constructed scenarios with a known-by-construction right answer) proved genesis
  handling, single- and convergent-origin tracing, the tie-break rule, and the
  sustain/blip distinction — all before ever pointing the detector at a real run.
- **`ex2.rs`** — additive: `FieldOverride` (six independent skip-flags, one per mutation
  branch) + `Overrides` (a `child_id`-keyed map) + `run_with_overrides`, sharing a private
  `run_impl` with the existing `pub fn run` (now a thin, provably-unchanged wrapper —
  re-verified `ex2_cpu_replay.rs`/`ex2_gpu_vs_cpu.rs` pass identically after the split).
  Reverting *one field* of *one birth*, not the whole birth, matters in practice: the real
  event found below shows a single birth mutating two fields at once (`repro_threshold`
  *and* `repro_promoter` together) — reverting the whole call would have confounded which
  change caused the detected effect.
- **`tests/ex4_counterfactual_replay.rs`** — proves the fork mechanism itself, independent
  of `lineage.rs`'s detection logic: revert one real field from one real birth, and confirm
  (a) every tick before that birth is byte-identical to baseline, (b) the reverted field now
  matches the parent's value, (c) the run's overall history hash still diverges afterward
  (the override wasn't a no-op).
- **`src/bin/ex4_lineage_report.rs`** — runs a real population, searches (seed ×
  K ∈ {3,5,10} × role) for a mutation-bearing plurality event, prints the full 6-field diff
  at the origin, reverts it, replays, and reports the honest outcome either way (causation
  confirmed, or a redundant-origins finding).

### Receipts

Grazer genome, `ex2_mutation_report.rs`'s same base config (8 initial organisms, 32×32
world, density 0.2, 2000 ticks, GPU engine), swept across the same 8 seeds
`cell80-life-findings.md`'s Finding 3 used (plus the running seed `0x5eed_1234_c311_80ff`
first). The first seed produced no sustained event at all; the second (`seed=1`) produced
one immediately:

| | value |
|---|---|
| role | `repro_promoter` |
| shift | pool index 37 → 33 |
| shift tick | 1,080 (K=5, sample every 20 ticks — 100-tick sustain window) |
| share at shift / peak in window | 35.3% / 41.6% |
| origins found | 1 (clean — no convergent-origin ambiguity this time) |
| origin birth | organism 2231 (parent 2059), tick 994 |
| origin diff | `repro_threshold: 198→192` **and** `repro_promoter: 37→33` in the same birth |
| pre-fork ticks identical after revert | true (every tick < 994) |
| event recurs after reverting just `repro_promoter`? | **no — causation confirmed** |

`cargo test -p cell80-life` (both platforms) and `cargo clippy -p cell80-life
--all-targets` are green, including the two new EX-4-specific tests and all of EX-0/1/2's
existing tests, unchanged.

### What this shows

- The full pipeline works end-to-end on a real run, not just the synthetic unit tests:
  hash → tree → sustained-event detection → backward-traced single origin → full-diff
  report → single-field revert → byte-identical-before-the-fork replay → the event's
  disappearance, all from one seed sweep with no cherry-picking beyond "first
  mutation-bearing event found."
- The full-diff design decision paid off immediately in practice, not just in principle:
  the real origin birth mutated *two* fields at once. Reporting only the flagged role would
  have hidden that `repro_threshold` moved in the same event — exactly the
  misrepresentation the design was built to avoid.
- The event is honestly modest, and reported as such: a 35–42% plurality shift among (at
  the time) dozens of competing pool indices, not a majority and nowhere near
  population-genetics fixation — consistent with EX-2's own dispatch-count receipts, and
  exactly the reconciliation the design doc's language needed.

### What this does *not* show

- **Only one event was traced end-to-end.** The report searches seeds until it finds a
  *mutation-bearing* candidate and stops there (by design — this is a demonstration that
  the mechanism works, not a survey of how many events a run contains). A convergent-origin
  case (>1 origin, which the synthetic tests already prove the machinery handles) was not
  exercised on real data this pass — the real run that came closest (`seed=0x5eed...`'s
  first candidate, before the search moved to `seed=1`) was a 6-origin, all-genesis
  reversion-to-baseline case, itself an honestly-reported outcome but not the flagship one.
- **The counterfactual reverted the *traced* origin only, not every conceivable path to the
  same value.** "The event no longer occurs" confirms this specific mutation was
  sufficient/necessary for *this* instance of the shift; it does not prove no other
  lineage could ever reach the same plurality by a different route in a longer run.
- **Numeric-field "fixation" is out of scope**, per the design decision — `decay_amount`/
  `repro_threshold`/`repro_give_pct` drift continuously and are not modeled as discrete
  winners here (see `cell80-life-findings.md` Finding 3 for that mechanism).
- **The K∈{3,5,10} sensitivity sweep is implemented and used to pick a candidate, but this
  section doesn't report the full comparative table** (how many events each K finds across
  all seeds) — the report binary optimizes for finding one clean, real demonstration, not
  for a systematic K-sensitivity census.

### Reproduce it

```
cargo test -p cell80-life --lib lineage                       # any platform, the 7 synthetic unit tests
cargo test -p cell80-life --test ex4_counterfactual_replay      # any platform
cargo run -p cell80-life --release --bin ex4_lineage_report     # macOS (Metal) only, ~1-2 min
cargo clippy -p cell80-life --all-targets
```

### What would raise confidence further

- Deliberately search for (or construct) a convergent-origin case in a real run, not just
  the synthetic test, to see the multi-origin report path exercised on GPU-produced data.
- Report the full K-sensitivity table (event counts at K=3/5/10 across all 9 seeds), not
  just the one candidate used for the demonstration.
- Extend the origin-tracing/diff report to numeric fields too, even though they're not
  modeled as discrete "fixation" events — a births-log-level diff is still meaningful for
  them.
- Run the counterfactual multiple times at different points to see whether *any* mutation
  reaching this same pool index would eventually be found again given enough ticks/seeds —
  i.e., is this specific mutation's disappearance a genuine dead end for the population, or
  just a delay before an equivalent one reappears independently.

## EX-5 — SOMA hand-off

**TL;DR: passed cleanly on the first real run, and it required zero new `cell80`/`rustrv32`
code — the multi-target RV32 export pipeline was already mature enough for this to be
integration work, not compiler work.** One real surviving predator from an EX-3 flagship run
had its full resolved genome (6 gene-cell choices) hash-attested and proven behaviorally
identical across the Z80 body, the RV32 body (the robot's target ISA), and the
CPU-reference interpreter — plus, as a bonus, the GPU body already proven throughout
EX-0–EX-3. The organism's `repro_promoter` had genuinely evolved away from its species'
starting cell (`is_ge` → `bit_is_set`, a real cell-swap mutation somewhere in its lineage,
not a hand-picked example) — so this isn't a demo of "a" cell exporting cleanly, it's a
demo of *this specific organism's evolved decision* exporting cleanly.

### What was built

- **`src/bin/ex5_soma_export_report.rs`** — runs a real EX-3 flagship simulation (seed 42,
  the same config `ex3_predator_prey_report.rs`/`ex3_arms_race_report.rs` validated as
  fully robust: 48×48, 60 grazers : 10 predators, density 0.3, `predator_satiation_ticks=20`,
  mutation on), picks the first surviving predator at the final tick, computes its
  `GenomeFields::hash()` (EX-4's lineage content-address, reused unmodified as a "genome
  digest"), reverse-resolves its 3 evolved roles to named cells via `role_pools.promoters`/
  `role_pools.movement` (plain indexing — no new lookup method needed), and for all 6 of its
  gene-cell choices (3 fixed: decay/eat/split; 3 evolved): compiles both bodies from the
  cell's real disk source (`Cartridge::compile`/`compile_rv32`), hash-attests them exactly
  `cell80/tests/cartridge_v10.rs`'s proven `one_cell_two_bodies_one_family` pattern
  (`family_hash` equal, `artifact_hash` differs, `from_bytes` round-trips), and runs all
  three bodies (Z80, RV32, CPU-reference; GPU too on macOS) over `DEFAULT_PROBES`, asserting
  bit-exact agreement. **Not macOS-gated** — `Rv32Runner`/`rustrv32::run_cell` is a
  pure-Rust RISC-V executor, not Metal-backed; only the GPU cross-check is a macOS-only
  bonus block that never gates the pass/fail verdict.
- **`tests/ex5_rv32_export.rs`** — pins the mechanism itself on two real disk cells
  (`is_gt`, `argmax3` — covering both arity-2 and arity-3 gene shapes) independent of a full
  flagship run: same hash-attestation + three-way-agreement assertions, fast (<0.1s) and
  fully deterministic, mirroring `tests/ex4_counterfactual_replay.rs`'s precedent of proving
  the mechanism apart from the exploratory report binary.

### Receipts

Seed 42, 3,000 ticks: 1,927 grazers, 500 predators survived. Organism `id=133288` (predator,
energy 83 at tick 2999) picked:

| field | value |
|---|---|
| genome digest (`GenomeFields::hash`, 8-hex prefix) | `acd9e94364d25cba` |
| `decay_amount` / `repro_threshold` / `repro_give_pct` | 1 / 254 / 54 |
| `hungry_promoter` | `is_gt` |
| `repro_promoter` | `bit_is_set` (evolved away from the species' starting `is_ge`) |
| `sense_move` | `argmax3` |

All 6 resolved cells (`sub_sat`, `add_sat`, `discount_percent`, `is_gt`, `bit_is_set`,
`argmax3`) attested identically:

| check | result |
|---|---|
| `family_hash` equal (Z80 body == RV32 body) | true, all 6 |
| `artifact_hash` differs (per-body identity, as expected) | true, all 6 |
| `from_bytes` round-trip of the RV32 body re-verifies | true, all 6 |
| Z80 == RV32 agreement over 20 `DEFAULT_PROBES` | true, all 6, zero mismatches |
| (macOS bonus) GPU == CPU-reference interpreter, same 20 probes | true, all 6 |

`cargo test -p cell80-life --test ex5_rv32_export` (any platform, <0.1s) and
`cargo run -p cell80-life --release --bin ex5_soma_export_report` (any platform, ~15s: most
of it the 3,000-tick flagship run) both green; `cargo test -p cell80-life` (both platforms)
and `cargo clippy -p cell80-life --all-targets` stay green, including every prior
experiment's tests unchanged.

### What this shows

- **The multi-target RV32 export path is mature enough that EX-5 was integration work, not
  compiler work.** Every API this needed — `Cartridge::compile_rv32`, `Rv32Runner`,
  `find_cell_file`, the family-hash/artifact-hash contract — already existed and worked
  exactly as `cell80/tests/cartridge_v10.rs` had already proven for hand-picked cells; zero
  lines changed in `cell80`, `rustrv32`, or `rustz80`.
- **The "genome IS a policy cell" framing holds up at the per-cell granularity the user
  chose.** An evolved organism's decision-making genome decomposes cleanly into named,
  independently-exportable, independently-hash-attestable cells — including a role that had
  genuinely mutated away from its species' baseline (`repro_promoter`: `is_ge` →
  `bit_is_set`), proving this isn't limited to genomes that happen to stay at their
  starting values.
- **The determinism spine now spans four independent execution bodies for the same
  organism's real decisions**: Z80, RV32, the CPU-reference interpreter, and (on macOS) the
  Metal GPU body — all agreeing bit-for-bit over the same probe inputs, extending EX-0–EX-3's
  "GPU ≡ interpreter" discipline one more step to "GPU ≡ interpreter ≡ RV32."

### What this does *not* show

- **This is per-cell attestation, not a single composed whole-organism RV32 artifact** — a
  deliberate, explicit scope decision (the user's choice between the two options offered).
  The tick engine's host-orchestrated control flow (axis-priority movement, contention
  resolution, predation-kill ordering) still lives in `ex3.rs`'s Rust code, not inside any
  cell; folding it into one exported program is real redesign work, not a prototype, and
  `composition.rs`'s existing machinery only does 2-cell arity-preserving wiring today, not
  the N-cell wiring this would need.
- **Cycle counts are not reported.** `Rv32Report.cycles` exists, but the RV32 cycle table
  stays provisional until the RP2350 `mcycle` co-sign (B4, per `rv32.rs`'s own module doc) —
  reporting a number here would misrepresent it as hardware-timed.
- **Composed/non-disk candidate cells (EX-2's cell-assembly operator) are out of scope.**
  The flagship config this pass used never extends the movement/promoter pools with
  composed candidates, so every resolved role was a real disk cell; exporting a composed
  candidate would need synthesizing valid `.cell` source text from its in-memory `Expr`
  representation first — not attempted here.
- **Only one organism, one seed.** This is a single demonstration that the seam works, not
  a survey of how often/well it works across the population — matching EX-4's own stated
  precedent (demonstrate on the first real candidate found, not an exhaustive sweep).

### Reproduce it

```
cargo test -p cell80-life --test ex5_rv32_export                  # any platform, <0.1s
cargo run -p cell80-life --release --bin ex5_soma_export_report   # any platform, ~15s
cargo clippy -p cell80-life --all-targets
```

### What would raise confidence further

- Export organisms from several seeds/ticks, not just one, to see whether every resolved
  role cell attests cleanly or whether some (e.g. a state-carrying or higher-arity cell, if
  the pools ever grow to include one) hits a gap the current six-cell genome doesn't exercise.
- Extend to a composed/non-disk candidate cell (EX-2 operator b) once one is actually
  adopted in a flagship-config run, to test the "synthesize `.cell` source from `Expr`"
  gap named above.
- Once B4 lands, re-report cycle counts as real, silicon-co-signed numbers instead of
  omitting them.
- Prototype the actual `CellHost`-by-body dispatch this would need to run a *whole tick's*
  decision sequence as one RV32 program — the natural next step toward the design doc's
  fuller "deploys to the RP2350" claim, once that upstream machinery lands.
