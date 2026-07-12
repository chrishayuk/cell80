# Deterministic Ecology: findings

Companion to `deterministic-ecology.md` (the design/pre-registration doc for EX-0…EX-5).
That doc says what each experiment would need to show; this one reports what running them
actually showed, with the receipts, one `##` section per experiment as they land — mirroring
the single multi-experiment design doc rather than one findings file per experiment.

Code lives inside `experiments/cell80-life/` (`src/rng.rs`, `src/contention.rs`,
`src/genes.rs`, `src/history.rs`, `src/pools.rs`, `src/ex0.rs`, `src/world2d.rs`,
`src/ex1.rs`, `src/ex2.rs`, `src/bin/ex1_sweep.rs`, `src/bin/ex2_mutation_report.rs`,
`tests/ex0_*.rs`, `tests/ex1_*.rs`, `tests/ex2_*.rs`) rather than a new crate — deliberate,
per the project's current preference to stay inside `experiments/` rather than promote to
a new workspace member while this stays speculative/off-roadmap.

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

## EX-2 — open-ended genome mutation (operator a: parametric + cell-swap)

**TL;DR: genome diversity emerges and grows under mutation exactly as expected, both gates
pass, and building this surfaced a real, previously-latent bit-exactness gap in EX-0/EX-1's
own machinery — now fixed.** Every organism now carries its own genome (three numeric
fields + three swappable-role pool indices); mutation on reproduction produces measurable,
growing diversity (dispatch-count-per-role climbs from 1 to 7–30 over 2,000 ticks); 95.7%
of births carry at least one role differing from the run's starting genome. Replay is
bit-exact and GPU agrees byte-for-byte with the CPU-reference interpreter, including the
new grouped-by-pool-index dispatch. This pass ships operator (a) only — parametric +
cell-swap, the doc's "known" control — per the explicit checkpoint in the design doc:
operator (b) (cell-assembly composition) is deferred to a follow-up pass.

### What was built

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
  (composed candidates will use `None`, though none exist yet — that's operator (b));
  `CompiledGene::from_funcs` (unused so far, ready for operator (b)); `batch_run_grouped`,
  the heterogeneous-cell-choice dispatcher (one GPU call per distinct pool index in use,
  not per organism), unit-tested against a naive per-organism loop.
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

- **Operator (b) — cell-assembly/bytecode-level mutation — is not built in this pass.**
  Per the design doc's explicit checkpoint, operator (a) alone is a legitimate stopping
  point; the pre-registered comparison ("does bytecode mutation reach strategies parametric
  mutation cannot") needs operator (b), which is a separate, larger follow-up.
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

### Reproduce it

```
cargo test -p cell80-life --test ex2_cpu_replay              # any platform
cargo test -p cell80-life --test ex2_gpu_vs_cpu               # macOS (Metal) only
cargo run -p cell80-life --release --bin ex2_mutation_report  # macOS (Metal) only, ~25s
cargo clippy -p cell80-life --all-targets
```

### What would raise confidence further

- A population-genetics-style analysis of which specific pool members become
  over-represented vs. a neutral-drift null, mirroring `cell80-life-findings.md`'s own
  flagged-but-undone control (mutation-disabled-after-founding) for its Finding 3/4.
- Confirm the dispatch-count-stays-cheap finding at a larger population scale than this
  pass's ~150–250 (EX-1's 10⁴–10⁵ organism runs, with mutation now turned on).
- Operator (b) itself — the actual pre-registered comparison this pass explicitly defers.
