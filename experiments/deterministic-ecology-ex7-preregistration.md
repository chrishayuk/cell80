# Pre-registration: EX-7 — where does mutational load start to dominate?

Status: **registered, nothing run.** Direct extension of EX-6
(`deterministic-ecology-ex6-preregistration.md`, results in
`deterministic-ecology-findings.md` `## EX-6`). EX-6's own "what would raise confidence
further" listed this experiment by name: *"a swap-rate sweep rather than a binary on/off, to
find where load starts to dominate — the D1/D2 reversal says the boundary is
population-size-dependent and locating it would turn a qualitative claim into a quantitative
one."*

## What EX-6 left unresolved

EX-6 established that this world has a ratchet and that the full mutation operator hides it:
at D2 (decay 6) recovery was 0.505 under numeric-only mutation and 0.331 with swaps enabled,
2/5 seeds below the drift null instead of 4/5. The proposed mechanism was deleterious swap
load against pools of 89 promoters and 43 movement cells.

But EX-6 also produced a result that does **not** fit a simple load story: **at D1 (decay 3),
full mutation *beat* numeric-only, 0.783 against 0.706.** If swaps were uniformly deleterious
that could not happen. EX-6's own honest reading was that load overwhelms selection only where
the population is too small to purge it (14–22 survivors at D2 against 64–88 at D1) — but that
was inferred from two points on a binary switch, which is exactly the kind of mechanism story
that sounds right and has not been tested.

A binary on/off cannot distinguish "dose-dependent load" from "some other difference between
the two modes." A sweep can.

## The claim being tested

**Recovery degrades monotonically with swap rate, and the boundary can be located.** If
recovery at D2 falls smoothly as the swap rate rises from 0 to 8%, EX-6's load mechanism is
supported and the sweep yields the crossover rate as a number. If recovery is flat in swap
rate, then EX-6's numeric-vs-full gap was not a dose effect and its stated mechanism is wrong
— **this experiment can falsify EX-6's explanation while leaving EX-6's headline (the ratchet
exists) untouched.**

## Design

- **Rungs:** D1 (`decay_amount` 3) and D2 (`decay_amount` 6) — the pair where the reversal
  appeared. D0/D3/D4 are not re-run; D4 was ruled not testable and D3 recovered ~0.18 in both
  modes.
- **Swap rate:** `swap_mutate_pct` ∈ **{0, 1, 2, 4, 8}**. 8 is the operator exactly as EX-2,
  EX-3 and EX-6's `Full` arm ran it.
- **Seeds:** 5, same base seed and flagship config as EX-6 (2000 ticks, 8 initial organisms,
  32×32, food_density 0.2, food_value 40, regrow_ticks 8).
- **2 rungs × 5 rates × 5 seeds = 50 runs.**

Metrics, all as defined in EX-6 so the numbers are directly comparable:

- **Primary:** mean `decay_amount` in the final living population, against the
  lineage-depth-matched clamped-random-walk null (10,000 replicates, fixed seed); recovery
  fraction `(start − observed) / (start − 1)`.
- **Churn:** births, starvations, and births per surviving organism.
- **Role fidelity:** fraction of the final population still carrying the curated role cells
  (`is_ge` reproduction promoter, `is_gt` hunger promoter, `argmax3` movement).

## Validity gate — checked before anything else is read

**`swap_mutate_pct = 0` must reproduce EX-6's `NumericOnly` numbers exactly**: D2 recovery
0.505 with 4/5 seeds below null, D1 recovery 0.706 with 5/5. The two are the same operator
expressed two ways — `rng::chance` is a pure function of `(seed, tick, child_id, stream)`
compared against a threshold, with no shared cursor, so a threshold of 0 never fires and
consumes nothing, which is exactly what skipping the branch does.

If those numbers do not match to three decimals, **the harness is wrong and every other number
in this experiment is void.** This is not a soft check.

Correspondingly, `swap_mutate_pct = 8` must leave `history_hash` byte-identical for every
pre-EX-7 config, verified the way EX-6 verified `MutationMode::Full`: capture the flagship
hash before the change, compare after. The landed replay tests compare two runs within one
binary and would not catch a behaviour change.

## Pinned bar

**Primary (rung D2):** Spearman rank correlation between swap rate and recovery fraction
across the five rates **≤ −0.9**, AND recovery at rate 8 lower than at rate 0 by **≥ 0.15**.
Both conditions.

- **Fails either → EX-6's load mechanism is not supported.** Registered conclusion: the
  numeric-vs-full gap at D2 is not a dose-dependent swap effect, and EX-6's mechanism
  paragraph needs withdrawing even though its headline result stands.
- With 5 points, ρ ≤ −0.9 means near-perfect monotone decline; the ≥0.15 margin stops a
  monotone-but-negligible slope from counting.

**The deliverable number:** the highest swap rate at which D2 recovery still reaches 0.50.
That is the "load starts to dominate above X%" figure this experiment exists to produce, and
it is reported whether or not the primary bar passes.

**Secondary (rung D1), registered as a prediction not a bar:** recovery should be flat or
*rising* in swap rate, reproducing EX-6's reversal. If D1 declines as steeply as D2, then
EX-6's population-size explanation for the reversal is wrong and the reversal was seed noise.

## Pre-registered predictions

- **P1 — D2 recovery declines monotonically (ρ ≤ −0.9).** Medium confidence. This is the first
  prediction in this line where I expect my own hypothesis to *hold*, which is itself worth
  flagging given the 0/3 record on emergence claims — the difference is that this one is a
  dose-response on a mechanism already observed at two points, not a new emergence claim.
- **P2 — D1 does not decline (ρ > −0.5).** Medium. Rests entirely on EX-6's two-point
  reversal, which could be noise across 5 seeds.
- **P3 — births per surviving organism rises superlinearly in swap rate.** Medium-high. The
  churn signature is stark (EX-6: 29,009 births against 579 with mutation off at D0).
- **P4 — the crossover sits between 2% and 4%.** Low confidence, stated so the number cannot
  be presented afterwards as expected.

## What this cannot show

- **It does not re-test EX-2.** Swaps and composition are what EX-2 studied: operator (a)'s
  reach beyond the parameter space needs swaps by definition, and operator (b)'s composed
  genes enter a population *through* swaps, so at low rates there is less of EX-2's own
  subject, not a cleaner measurement of it. This sweep gives the churn-versus-load curve that
  contextualises the regime EX-2's numbers were measured in — nothing more. A real EX-2 re-test
  would need the composition machinery held fixed while load varies, and its own
  pre-registration.
- **It does not speak to EX-3.** That confound is being tested separately by the numeric-only
  arm of the arms-race detector.
- **One field, still.** `decay_amount` was chosen in EX-6 because it is the clean monotone
  case; nothing here transfers to the non-monotone fields.
- **Nothing about open-endedness.** Recovery toward a known optimum is not cumulative
  evolution, and locating a load threshold does not change that.

## Build list

1. `swap_mutate_pct: u64` on `RunConfig2DGenome` and `RunConfig3`, threaded into
   `ex2::mutate`. Default 8 must be byte-identical to today.
2. Extend `ex6_degraded_seed_report.rs` into the rate sweep (or a sibling binary), reusing the
   existing drift null and depth machinery unchanged.
3. Role-fidelity and churn reporting — the modal-role diagnostic EX-6 added already covers
   most of it.
4. Spearman ρ over the five rates.
5. Extend `tests/ex6_cpu_replay.rs` with the rate-0 ≡ `NumericOnly` equivalence assertion, so
   the validity gate is a test rather than a one-off observation.

Nothing runs until this document is committed.
