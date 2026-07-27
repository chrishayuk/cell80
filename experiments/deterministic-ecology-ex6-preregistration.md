# Pre-registration: EX-6 — was the ecology seeded at a local optimum?

Status: **registered, nothing run.** Follow-on to the doc-17 programme
(`deterministic-ecology.md`, EX-0…EX-5, complete 2026-07-12), written after a cross-experiment
synthesis on 2026-07-27 raised a question the original programme never asked. Same discipline
as every experiment in this line: hypothesis, controls, nulls, and a numeric bar committed
before anything runs.

## Where this came from

Laying all nine substrate experiments side by side splits the verdicts by *kind of claim*:
checkable substrate properties passed 4/4 (EX-0 replay, EX-0 parity, EX-4 attribution, EX-5
export), targeted search went 1/2, and **untargeted emergence went 0/3** — EX-1's regime split
was a dimensionality artefact, EX-2's composed genes came out uniformly *less* fit than their
parents, EX-3's arms race failed a permutation null.

Reading EX-2 next to C1 (`cell-superoptimization-findings.md`) suggests one explanation for
the 0/3 that neither experiment could see alone. C1: the authored library is near-optimal,
search cannot beat hand-written cells. EX-2: the parent genome is near-optimal, composition
cannot beat it. Those are the same result in two domains — **hand-curated starting points sit
at a local optimum, and undirected recombination from them goes downhill.**

If that is right, the ecology never had anywhere to climb. It was seeded with already-good
cells, so the only available moves were down, and selection correctly rejected them — which is
precisely the `argmin3` purge working as designed. An ecology started at a local optimum
structurally cannot demonstrate cumulative evolution no matter how long it runs, and the 0/3
would be a fact about the *starting point*, not about the world.

That is a hypothesis assembled after the fact from two experiments' results, which is exactly
the kind of story that feels true and isn't. EX-6 is the control that was never run.

## The claim being tested

**Seed the world below the curated optimum and selection will climb back.** If it does, the
machinery for cumulative gain is present and the starting point was the ceiling. If it does
not, the world genuinely lacks a ratchet, and the 0/3 is a fact about the ecology rather than
about how it was seeded.

## Why `decay_amount` is the primary axis

The grazer genome (`genomes/grazer.json`) carries four numeric fields and three swappable role
cells. `decay_amount` is the one field where the test is clean:

- **Monotone.** Lower decay is strictly less energy lost per tick. There is no trade-off to
  argue about, unlike `repro_threshold` (reproduce early and weak vs. late and strong) or
  `repro_give_pct`.
- **Its optimum sits at the bound.** The curated value is 1 and `DECAY_BOUNDS` is `(1, 6)`
  (`ex2.rs`), so 1 is the best reachable value, not merely a good one.
- **Reachable in single steps.** The numeric operator draws a step in `{-1, 0, +1}` at 25% per
  birth, so 6 → 1 is five steps along a monotone gradient with no epistasis.

If selection cannot climb *that*, no weaker claim about ratchets survives. It is the easiest
hill this world contains.

## The degradation ladder

| rung | genome change from curated | why |
|---|---|---|
| **D0** | none (curated grazer) | ceiling reference |
| **D1** | `decay_amount` 1 → 3 | mild, monotone |
| **D2** | `decay_amount` 1 → 6 | **primary rung** — maximum degradation still inside `DECAY_BOUNDS` |
| **D3** | `decay_amount` 1 → 6, `repro_threshold` 200 → 400 | two fields, tests whether recovery survives a second deficit |
| **D4** | `sense_move` `argmax3` → `argmin3` | categorical, known-bad (move *away* from food) |

## The mutation-mode factor, and why it was added

Measured against the current library: the discovered promoter pool is ~89 cells and the
movement pool ~43 (approximate — the run reports the exact figures from `discover_pools`).
Both are far larger than when EX-2 and EX-3 ran. The swap operator fires at 8% per birth and
`rng::pick_other_index` always lands on a *different* pool member, so a swap is overwhelmingly
likely to replace a working role cell with a useless one. **Swap mutation in this library is
close to pure deleterious load, and that load grows every time the library grows.**

That raises a rival explanation for the missing ratchet which has nothing to do with local
optima: gains may be real but swamped by mutational load. Separating the two costs one factor:

- **Off** — children clone parents exactly (mirrors `ex3::RunConfig3::mutation_enabled`, the
  existing precedent). A flat floor: the genome cannot change, so recovery is definitionally
  zero.
- **Numeric-only** — numeric jitter fires, all three swap branches suppressed.
- **Full** — the operator as EX-2/EX-3 ran it.

Design: **5 rungs × 3 modes × 5 seeds = 75 runs**, flagship config
(`seed 0x5eed_1234_c311_80ff` + seed index, 2000 ticks, 8 initial organisms, 32×32,
food_density 0.2, food_value 40, regrow_ticks 8) — matching `ex2_mutation_report.rs` so numbers
are comparable to EX-2's.

## The primary metric is the genome, not the population

Recovery is measured as **the mean `decay_amount` across the living population at the final
tick**, not as population size or birth count. Population fitness confounds "the genome
improved" with "the world happened to be easier this run"; the genome is directly observable
and is the whole point of the substrate. Population statistics are reported as secondary.

Recovery fraction = `(start − observed) / (start − 1)`, so D2 going 6 → 1 is 1.0 and 6 → 6
is 0.0.

## The drift null — the control that decides this experiment

**A clamped random walk on `[1, 6]` drifts toward the middle on its own.** Starting at 6 with
symmetric ±1 steps and clamping, mean `decay_amount` falls toward roughly 3.5 with **no
selection whatsoever**. So "final mean decay ≈ 3.5" is what pure drift predicts, and reading it
as recovery would be a textbook wrong-sign claim of exactly the kind that has already cost this
programme once (`frequency-needs-drift-baseline`).

The bar is therefore against the drift null, never against the starting value:

**Lineage-depth-matched random-walk null.** For every organism alive at the final tick, take
its depth `D` in the realized lineage tree (available from the birth log, EX-4's machinery).
Under mutation without selection its `decay_amount` is a clamped random walk of `D` steps from
the rung's starting value, each step firing with probability 0.25 and drawn uniformly from
`{-1, 0, +1}`. Monte-Carlo that walk over the observed depth distribution (10,000 replicates,
fixed seed) to get the null mean and its 95% interval. Selection is credited only where the
observed mean sits **below the null interval**.

This deliberately mirrors EX-3's permutation null, which is the control that killed the arms
race. It is the same discipline pointed at a claim I would prefer to be true.

## Pinned bar

**Primary (rung D2, mode Full):** observed final mean `decay_amount` below the drift null's
95% interval in **≥ 4 of 5 seeds**, with a mean recovery fraction **≥ 0.50**. Both conditions.

- Fails either → **no ratchet on the easiest hill in the world.** Registered conclusion: the
  0/3 on emergence is a fact about the ecology, not about its starting point, and the
  local-optimum story above is wrong.
- **Numeric-only recovers but Full does not** → the ratchet exists and is swamped by
  deleterious swap load. That is a different finding with a different fix, and it implicates
  library growth directly: the swap pool grows every time the library does.
- **Neither recovers** → the numeric operator itself is too weak, or selection pressure on
  decay is too shallow at this config to act on. Reported as such, not as a verdict on ratchets
  in general.
- **Off** must show recovery fraction exactly 0.0 in all seeds. If it does not, the harness is
  broken and every other number is void.

**Extinction rule, fixed now.** A seed whose population reaches zero before the final tick is
reported as extinct and excluded from that rung's recovery mean. A rung with **> 2 of 5** seeds
extinct is reported as *not testable at this degradation* — not as a failure to recover.
Degrading a world until it dies and calling that "no ratchet" would be a measurement artefact,
and D3/D4 are the rungs at risk.

## Pre-registered predictions

- **P1 — D2/Full fails the bar.** Medium-high confidence. This is a prediction that my own
  hypothesis is wrong; the 0/3 record is the base rate and I am not exempt from it.
- **P2 — D2/Numeric-only beats D2/Full on recovery fraction.** Medium-high. The swap-load
  argument stands on measured pool sizes, independent of whether anything recovers.
- **P3 — D4 does not recover in any mode.** High. Returning to `argmax3` specifically means
  hitting 1 of ~43 pool members; drift essentially cannot, and selection can only act on it
  once it appears. D4 mostly measures pool size, and is registered as such *before* seeing it,
  so a D4 failure cannot later be told as a ratchet story.
- **P4 — D3 and/or D4 trip the extinction rule.** Medium.

## What this cannot show

- **Nothing about open-endedness.** Climbing back to a known optimum is recovery, not
  cumulative evolution. A pass would establish that gains can accumulate *at all* in this
  world — the weakest possible version of the claim, and still more than the programme
  currently has.
- **Nothing about the other three numeric fields**, which are non-monotone; `decay_amount` was
  chosen precisely because it is the clean case, and a result there does not transfer.
- **It cannot rescue EX-3.** A ratchet on a single monotone field says nothing about whether
  predator and prey traits couple.
- **The local-optimum reading stays post-hoc even if EX-6 passes.** EX-6 tests a consequence of
  that story, not the story itself; "the curated genome sits at a local optimum" is never
  directly measured here.

## Build list

1. `MutationMode` on `RunConfig2DGenome` (Off / NumericOnly / Full), mirroring `ex3`'s existing
   `mutation_enabled` precedent. **`Full` must leave `history_hash` byte-identical** for every
   existing EX-2/EX-4 config — the landed replay tests are the check, and a hash change means
   the field was threaded wrong, not that the tests need updating.
2. Update the handful of `RunConfig2DGenome` construction sites.
3. `src/bin/ex6_degraded_seed_report.rs` — the ladder, the three modes, five seeds.
4. The lineage-depth-matched drift null (Monte Carlo, fixed seed).
5. `tests/ex6_cpu_replay.rs` — same replay discipline as every other EX.

Nothing runs until this document is committed.
