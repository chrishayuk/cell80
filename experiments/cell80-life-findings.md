# Cell80 Life: findings from the first mutation/selection runs

Companion to `cell80-life.md` (the design/status doc) and the code in `cell80-life/`. That doc
tracks *what's built*; this one tracks *what running it actually showed*, with the numbers
that back each claim — since "what's this telling us" deserves receipts, not just a summary.

## TL;DR

Composing six unrelated, already-curated stdlib cells (`sub_sat`, `is_gt`/`is_ge`, `add_sat`,
`argmax3`/`argmin3`, `discount_percent`) into a tick loop produces a population that forages,
starves, reproduces, and goes locally extinct — with no new cell code. Making the numeric
thresholds and cell choices per-organism and mutable on reproduction turns that from "a fixed
ecology" into something that visibly *selects*: a deliberately-planted bad mutation
(`argmin3`, move away from food) gets reintroduced by mutation and purged back out repeatedly,
rather than drifting freely or fixing.

## What was tested, in order

1. **Composition** — decay/eat/movement/reproduction each call a real `.cell` cartridge
   through a `CellHost`, not a Rust closure.
2. **Genome-as-data** — which cell backs each role, plus the tunable numbers, load from a
   `.genome.json` file instead of being hardcoded.
3. **Mutation** — each organism carries its own copy of the tunable genome; a child's copy is
   the parent's, mutated (numeric drift ~25%/field/birth, rarer ~8% swap to a same-signature
   sibling cell).

## Finding 1 — composition alone produces real ecological dynamics

With the `grazer.json` genome (decay=1, repro_threshold=200, repro_give_pct=50%, no mutation
needed to see this), a 300-tick run with 2 starting organisms settles into a stable population
of roughly 8–12, oscillating with the two food-rich zones in the 1D world. This isn't
surprising in hindsight, but it wasn't guaranteed — it means the 6 cells involved, none of
which were designed with "organism" in mind, compose cleanly enough to sustain a population
rather than immediately starving out or exploding without bound.

## Finding 2 — the genome file is causally load-bearing, not cosmetic

Swapping `grazer.json` for `rapid_reproducer.json` (decay=2, repro_threshold=90,
repro_give_pct=30%) with **zero code changes** shifts the dynamics from steady-state to
boom-bust: population cycles between 2 and 6 rather than settling, with roughly 3x the births
and deaths over the same 300 ticks (22/14 vs 64/60). The JSON file is doing real work, not
just relabeling identical behaviour.

## Finding 3 — mutation produces selection, not just noise

This is the one worth being careful about, so here's the actual data, not just the claim.

**Setup:** `grazer.json`, 2000 ticks, 8 different RNG seeds (`1,2,3,4,5,42,999,123456`).
`genome avg` is the population-mean of each field, sampled every 20 ticks and printed at the
end of the run.

| seed | final n | births | deaths | decay | thresh | give | hungry[is_ge] | repro[is_ge] | move[argmin3] |
|-----:|--------:|-------:|-------:|------:|-------:|-----:|---------------:|---------------:|---------------:|
| 1      | 11 | 150 | 141 | 1 | 200 | 49% | 0/11 | 10/11 | 0/11 |
| 2      | 12 | 150 | 140 | 1 | 199 | 49% | 0/12 | 10/12 | 0/12 |
| 3      | 12 | 150 | 140 | 1 | 199 | 49% | 0/12 | 10/12 | 0/12 |
| 4      | 12 | 150 | 140 | 1 | 200 | 50% | 0/12 |  9/12 | 1/12 |
| 5      | 12 | 150 | 140 | 1 | 200 | 50% | 0/12 |  9/12 | 1/12 |
| 42     | 11 | 150 | 141 | 1 | 200 | 49% | 1/11 | 11/11 | 0/11 |
| 999    | 11 | 150 | 141 | 1 | 199 | 50% | 1/11 | 11/11 | 0/11 |
| 123456 | 12 | 150 | 140 | 1 | 199 | 49% | 2/12 | 12/12 | 0/12 |

Confirmed the seed argument actually changes the run (not just relabels it): `diff`ing seed
1 vs seed 2's full output shows the two logs are identical only through the t=0 header line —
every stats line from tick 20 onward differs.

What stands out:

- **`decay_amount` never moves.** Across all 8 seeds and the full 2000-tick/~150-birth run,
  the population average stays at exactly 1 (the legal minimum) — never observed at 2, despite
  a 25%-per-birth mutation chance that should attempt roughly 35–40 increase-mutations over
  that many births. Decay is a permanent per-tick tax on every organism carrying it; the
  implication is that any uptick is corrected fast enough that it never survives to the next
  20-tick sample.
- **`repro_threshold` and `repro_give_pct` stay in a tight band around their starting values**
  (197–201 out of a legal 50–400 range; 48–50% out of a legal 10–90% range) rather than
  wandering across the space a truly neutral trait would explore given ~150 mutation
  opportunities. That's consistent with stabilizing selection near 200/50% — though see the
  caveat below.
- **`move[argmin3]` (seek away from food — planted as a clearly bad mutation) gets
  repeatedly introduced and purged, never fixes.** Tracing seed 1's full time series: it
  appears at tick 60 (via mutation), holds at 10–17% of the population through tick 240, drops
  to 0 by tick 260 and stays there for 400 ticks, then reappears at 660, disappears again by
  760, reappears at 940. That's mutation-selection balance playing out visibly across a single
  run, not a one-time event.
- **`hungry_promoter` (`is_gt` vs `is_ge`) shows no consistent direction** — 0, 0, 0, 0, 0, 1,
  1, 2 out of 11–12 across the 8 seeds. Expected: for this world, the two differ only on the
  `food_here == 0` edge case, which is behaviourally almost a no-op either way — closer to
  neutral than the movement swap.
- **Aggregate turnover (births/deaths/final population) is nearly identical across all 8
  seeds** even though each seed's specific mutation history differs. The ecology's macro
  equilibrium for a given starting genome looks robust to exactly which mutations happen to
  fire, with variation concentrated in which near-neutral variant is momentarily common.

A second genome (`rapid_reproducer.json`: decay=2, threshold=90, give=30%, ~400 births/2000
ticks) shows `thresh` in [87,92] and `give` in [28,31%] — again a tight band around *its own*
starting point, not a drift toward grazer's 200/50%. Each starting genome seems to settle near
its own basin rather than every run converging on one global optimum. Its `decay_amount` is
covered properly in Finding 4 below — an earlier version of this doc characterized it from a
single final-tick reading, which turned out to be misleading (see the correction there).

## Finding 4 — a dose-response curve: how bad a starting mutation is predicts whether selection can save it

Five variants of the grazer genome, identical except `decay_amount ∈ {1,2,3,4,6}`
(`genomes/decay_test_2.json`, `_3.json`, `_4.json`, `decay_extreme.json` for 6 — 1 is
`grazer.json` itself), each run 2000 ticks × 3 seeds. The first pass at this used only the
final tick, which was a mistake for a population this small (see the correction above) — this
instead counts, across all 101 sampled ticks, what fraction show the *original* decay value
vs. a *lower* (corrected) one:

| start | births (typ.) | final pop | samples @ improved value | samples @ start value |
|---:|---:|---:|---:|---:|
| 1 (grazer) | 150 | 11–12 | n/a — already at the legal floor | 101/101 @ 1 |
| 2 | 114 | 6 | 34/101 @ 1 | 67/101 @ 2 |
| 3 | 76 | 2–3 | 19–21/101 @ 2 | 80–82/101 @ 3 |
| 4 | 38 | 2 | 6–7/101 @ 3 | 94–95/101 @ 4 |
| 6 | 0 | 0 (extinct ~tick 97) | 0/101 — no births ever occurred | 101/101 @ 6 |

Each row's split is near-identical across all 3 seeds (e.g. decay=2 gives exactly 34:67 for
seeds 1, 2, and 3). It also reappears, unprompted, in `rapid_reproducer` — a genome with a
completely different threshold, give-pct, and ~4x the turnover, which also happens to start at
decay=2 and lands at 35:66. That's a real, specific number showing up twice from two
mostly-unrelated setups, not a coincidence of small sample size.

Two things this shows:

- **Correction ability degrades smoothly as the starting mutation worsens** — 34% → 20% →
  6–7% → 0%. It tracks total births almost exactly (114 → 76 → 38 → 0): a worse `decay_amount`
  doesn't just cost the individual energy, it shrinks the population, which independently
  weakens selection's ability to find and fix a rescuing mutation before either the lineage
  dies out or drift loses it in a population of 2–6. A plausible mechanistic read for why the
  same ~1/3 ratio shows up twice at decay=2 regardless of the rest of the genome: standard
  population-genetics theory says the equilibrium frequency of a recurring mildly-deleterious
  mutation depends mainly on the mutation rate and its fitness cost, not on unrelated traits —
  which is exactly what "same ratio, different genome" would predict. Plausible, not proven
  here — would need many more seeds and an isolated sweep of just the mutation rate to confirm
  rather than assert.
- **There's a cliff, not just a slope.** At decay=6, all three seeds go extinct by tick ~97
  with zero births — both starting organisms die before either one ever reproduces, so
  mutation never gets a single chance to act. This is a real boundary on the whole approach:
  mutation-and-selection can only rescue a lineage that survives long enough to create
  variation in the first place. There's no such thing as a correction arriving one generation
  too late here, because there is no later generation.

## What this does *not* show (important)

- This is a tiny, single-species, 1D world (24 cells) with one food distribution. It's a
  working demo of the mechanism, not a rigorous artificial-life benchmark — don't read these
  numbers as "cell80 proves evolution works," only as "this particular toy shows
  selection-like behaviour on the axes tested."
- "Selection" here is read off population-average snapshots every 20 ticks, not a formal
  fitness/selection-coefficient measurement. The `argmin3` purge story and the Finding 4 dose
  curve are the most direct evidence; the threshold/give bands are suggestive but could partly
  reflect "2000 ticks / a ±10-±5 step size isn't enough to escape the starting basin" rather
  than "200/50% is a true optimum."
- Populations here are tiny (2–12 organisms). At that size, genetic drift (an unlucky death,
  regardless of fitness) is not a rounding error — it's plausibly a big part of *why* the
  dose-response splits in Finding 4 aren't 0%/100% cliffs. This doc treats the recurring exact
  ratios as evidence of a real equilibrium, and floats a population-genetics explanation for
  why — but hasn't run the control that would nail it down (e.g. rerun decay=2 with mutation
  disabled after founding, to see how much of the 34% is drift on an already-mixed population
  vs. mutation continually reintroducing 1).
- The inspectability the wider Cell80 Life pitch promises — reading *why* a specific lineage
  died from its own cell `Report`s (cycles, halt reason, touched memory) — isn't exercised at
  the individual-organism level yet, only at the population-summary level printed here.

## Reproduce it

```
cargo run -p cell80-life -- <ticks> [genome.json] [seed]

# Finding 3's seed sweep:
for seed in 1 2 3 4 5 42 999 123456; do
  cargo run -p cell80-life --quiet -- 2000 experiments/cell80-life/genomes/grazer.json $seed
done

# Finding 4's dose-response sweep (decay_amount = 1, 2, 3, 4, 6):
for g in grazer decay_test_2 decay_test_3 decay_test_4 decay_extreme; do
  for seed in 1 2 3; do
    cargo run -p cell80-life --quiet -- 2000 experiments/cell80-life/genomes/$g.json $seed
  done
done
```

## What would raise confidence further

- A proper multi-seed statistical summary (mean ± spread over dozens of seeds, not 8) for the
  threshold/give-pct bands, and for the Finding 4 dose-response splits.
- The drift-vs-selection control described above: rerun decay=2 with mutation switched off
  after founding, to separate "mutation keeps reintroducing 1" from "drift alone explains the
  34%" once a mixed population already exists.
- Test the population-genetics explanation for the recurring ~1/3 ratio directly, by varying
  `NUMERIC_MUTATE_PCT` in isolation and checking whether the equilibrium fraction moves the way
  the theory predicts.
- Per-organism `Report` inspection on death (cycles spent, which promoter fired last) — the
  actual "read why" payoff the design doc's pitch is built on, not yet wired up here.
- Per-organism `Report` inspection on death (cycles spent, which promoter fired last) — the
  actual "read why" payoff the design doc's pitch is built on, not yet wired up here.
