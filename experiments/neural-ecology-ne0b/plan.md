# NE-0b development: plasticity cadence and operating stability

**Development plan v1, 2026-09-16. Fixed before these development runs.**

NE-0 completed with G0/G2/G5 passing and G1/G3/G4/G6 failing. Plastic
underperformed Frozen in every evaluation seed. Plastic and Yoked had 57–65%
of weights at a bound; Yoked firing exceeded 0.35. Saturation is an observed
failure, not an established cause of prediction loss.

This study asks only whether less frequent application of the same local rule
can meet the original activity and persistent-movement criteria. It does not
test learning, generalisation, or the mechanism of a successful learner. It is
not the proposed NE-1 study. NE-0 and its implementation seal remain unchanged.

## Fixed screen

| Item | Value |
|---|---|
| Development seeds | 3001, 3002, 3003; no replacements |
| Update intervals K | 1, 16, 256, 4096; all 12 seed/candidate combinations |
| Adaptation | 50,000 ticks |
| Reset observation R | Fresh transient reset, 256 warm-up + 10,000 scored activity ticks |
| Arms | Frozen, Plastic, Yoked |
| Node rule, local delta rule | Exactly the sealed NE-0 cells |
| Topology, initial weights, sensory encoding | Exactly NE-0 construction on new seeds |
| Grammar | Exactly NE-0; namespaces `grammar/A` and `grammar/NE0B-R` |
| RNG | Original addressed `NE0-v1/{seed}/...` generator; disjoint seed IDs |
| Statistical endpoint | None: this is a development operating-regime screen |

All intervals within a seed share the same graph, weights, yoke permutation,
initial state and streams. No new neuron parameter, homeostatic term, weight
bound, threshold or learning-rule coefficient is introduced.

At zero-based adaptation tick t, compute/apply the original local deltas only
when `(t + 1) % K == 0`. On other ticks every arm requests zero. There is no
fractional accumulator and no storage of skipped updates. Thus K=1 is the
original schedule. K=16/256/4096 have 3,125/195/12 update opportunities. This
changes exposure to plasticity and its cadence; it is not claimed equivalent to
a continuous small learning rate. Periodic phase-locking remains a limitation.

The Yoked arm receives the same fixed per-sign permutation of Plastic's requested
updates, at the same eligible ticks. Bounds and movement auditing are unchanged.
Frozen never applies a delta. During R all arms' weights are frozen. Weights-only
serialization/reconstruction and idempotent transient reset preserve persistent
bytes and zero all neuron state. R uses the same stream for all candidates/arms
in a seed. No targets, probe fitting, test loss, or accuracy enter selection.

## Fixed acceptance and selection

A candidate is viable only if **all three development seeds** satisfy:

1. Exact execution parity: independent checked-i64 reference, Cell80-generated
   CPU, Metal and repeated Metal agree on every tick's currents, nodes, requested
   deltas and committed networks through A and R. Same-seed Frozen histories must
   also agree across K. Reset and frozen-weight assertions pass.
2. Every arm has mean spike fraction in [0.01, 0.35] and at least 50% of units
   active at least once, both in the last 10,000 A ticks and scored R ticks.
3. Every arm has at most 25% endpoint weights after A.
4. Plastic has at least 1% of weights different from initial weights, and mean
   absolute final-minus-initial change is at least 0.10 integer units.
5. Plastic and Yoked have positive requested/realised movement per sign;
   realised movement relative gap and clipped/requested fraction gap are both
   at most 0.05. Requested delta histograms agree per tick/sign.

Choose the smallest K satisfying all criteria. If none does, report
`NO_VIABLE_CANDIDATE`; do not extend the grid or soften the thresholds. This
deterministic development selection does not establish an optimal cadence or
generalisation to unseen seeds. List every failure and all twelve rows.

Execution failures yield `INVALID_OR_INCOMPLETE`, not physiological failure.
No candidate is rejected early on activity. Complete the entire fixed grid.
Interrupted attempts stay on disk; any rerun uses the same seed/K in a new
directory and records the reason. There is no automatic mid-phase resume.

## Engineering, provenance and boundaries

Use a separate Cargo workspace and artifact directory. Import the existing NE-0
library without changing it. Record hashes of this plan, implementation, lockfile,
binary, inherited NE-0 seal and all inherited sealed files before execution;
commit that manifest before the screen. Recheck hashes at run entry and completion.
Run the inherited 700 primitive fixtures and new cadence/selection/reset checks.

Each candidate process owns its four independent worlds; run at most three
candidate processes concurrently. Record tick hashes, full checkpoints every
10,000 ticks and boundaries, activity, update opportunities, per-sign requested /
realised / clipped movement, final weights, timing, and every process exit. A
failed execution is preserved. No CUDA claim or isolated throughput claim is made.

After all twelve runs, compare Frozen trajectory hashes across K and publish a
compact development report. A viable candidate is only eligible for a new
learning protocol. Confirmatory seeds, hypotheses, probe choices and statistical
criteria must be frozen separately before any new evaluation streams are
generated. This study allocates and accesses no new confirmatory seeds.
