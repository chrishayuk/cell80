# NE-0c: confirmatory temporal learning at K=4096

**Protocol v1.0, 2026-09-16. Fixed before NE-0c evaluation data or inference.**

## Question and inherited specification

Do activity-dependent local updates at the development-selected cadence produce
persistent changes that improve held-out temporal prediction over Frozen and
Yoked after transient-state reset?

This protocol incorporates [NE-0 v1.0](../neural-ecology-ne0-preregistration.md),
SHA-256 `f526a743eaefa4adf2a7f56721a5cfbcca3d4c502c64a54366b5c81c0eeb18d6`,
as the exact definition of neuron dynamics, local delta rule, topology generation,
initialization, arithmetic, clipping, canonical ordering, input encoding, arm
semantics, grammar, feature representation, probe, controls and G0–G6. The explicit
changes below take precedence. No parameter search is part of NE-0c.

NE-0b's fixed development screen selected K=4096 as its only viable candidate
across seeds 3001–3003. This selection used activity and movement, not prediction
scores. NE-0's negative result and NE-0b's development findings remain unchanged.

## Fixed design

| Item | NE-0c value |
|---|---|
| Evaluation seeds | 4001–4010 inclusive; ten paired systems, all mandatory |
| Engineering validation | Reuse development seed 3001 only; no tuning |
| Units / edges | 1,024 / 16,384; 16 incoming edges per unit |
| Source signs | Units 0–767 excitatory; 768–1,023 inhibitory |
| Weight magnitudes | Integers 0–32, initialized uniformly on 1–16 |
| Adaptation A | 50,000 ticks |
| Update cadence | Eligible iff `(t + 1) % 4096 == 0` at zero-based A tick t |
| Update opportunities | Exactly 12 per A stream; no accumulated skipped updates |
| B probe-fit stream | 256 warm-up ticks, then 10,000 scored examples |
| C test stream | Independent reset, 256 warm-up ticks, then 10,000 examples |
| Arms | Frozen, Plastic, Yoked, Memoryless |
| Primary endpoint | Per-seed mean C negative log likelihood, nats |
| Minimum mean effect | 0.020 nats for each registered contrast |

The existing addressed RNG prefix `NE0-v1/{seed}/...` is retained. New seed IDs
separate NE-0c from all earlier data. Namespaces remain `grammar/A`, `grammar/B`,
`grammar/C`, per-target topology, per-edge initial weight, per-sign yoke and
conditional-state shuffle. The graph, initial state and streams are identical
across arms within a seed. Every noneligible A tick and every B/C tick requests
zero updates. Plastic's eligible requested deltas are computed by the unchanged
Cell80 rule; Yoked receives their unchanged fixed per-sign derangement.

Grammar: four symbols; initial ordered pair uniform, then
`x[t+1]=(x[t-1]+x[t]+z) mod 4`, with P(z=0)=0.7 and P(z=1,2,3)=0.1 each.
Current-symbol Bayes loss is ln(4)=1.3862943611198906; history Bayes loss is
0.9404479886553263. Score the two-symbol oracle on actual C targets too.

## Primary sequence and state semantics

1. A starts all recurrent arms from identical initial weights and zero state.
2. Freeze every synapse after A. Save both a weights-only checkpoint and a separate
   full diagnostic snapshot. Learned parameters are only synaptic magnitudes;
   thresholds, topology and signs remain fixed.
3. B reconstructs the weights-only checkpoint in a fresh process. Reset membrane,
   refractory, spike and trace fields to zero; verify preservation and idempotence.
4. Apply the common unscored warm-up and export the registered trace/current-symbol
   features. Each arm fits its own fixed probe on B only.
5. Freeze probes before generating C. C independently reconstructs the weights-only
   checkpoint and repeats the reset/warm-up procedure. It inherits no B activity.
6. Score C with frozen probes. Perform the registered conditional shuffled-state
   diagnostic within current-symbol groups without refitting.

The probe is unchanged: 1,024 traces divided by 255×32 plus current-symbol one-hot;
Memoryless zeros the trace coordinates. Four-class float64 linear softmax with
intercept, zero initialization, L2=0.01 on weights only, exactly 2,000 full-batch
gradient steps at learning rate 0.5, stable log-softmax, one numerical-library
thread. No alternative probe, normalization, early stopping or calibration.

## Secondary no-reset diagnostic

Only after primary C and the primary replay checks, run a separate diagnostic.
For each of B and C independently, load the **full final-A snapshot**, including
neuron state, and omit transient reset. Freeze weights throughout. Use precisely
the same respective B/C symbols, 256-tick warm-up, feature representation and
probe-fitting algorithm as primary. Fit separate diagnostic probes on diagnostic
B; freeze them before diagnostic C. C starts from final A, not final diagnostic B.
This paired branching isolates the presence of final-A transient state at stream
entry. It is not a continuous A→B→C training trajectory or a new grammar.

Report all diagnostic per-seed losses and Plastic–Frozen/Plastic–Yoked differences,
plus diagnostic-minus-primary losses. These are descriptive secondary outcomes:
no confirmatory p-values, candidate selection, additional gate, or rescue of the
primary claim. Identical replay guarantees apply to these simulated phases, but
they are explicitly not persistence tests. Warm-up can erase transient effects;
absence of a diagnostic difference is not evidence that transient state is never
useful. A diagnostic advantage without primary success is only consistent with
state-dependent performance; it does not establish durable synaptic acquisition.

## Statistics and gates

The seed is the experimental unit, n=10. Positive differences favor Plastic:
`d_F=L_F-L_P`, `d_Y=L_Y-L_P`, `d_M=ln(4)-L_P`.

Plastic versus Frozen and Plastic versus Yoked are the two primary causal
contrasts. Retain NE-0's G6 current-symbol contrast and its original joint
Bonferroni family of three tests, unchanged. For each contrast, let k count
strictly positive seed differences, with ties nonpositive. The one-sided exact
sign-test p is `sum(comb(10,j), j=k..10)/1024`; adjusted p=min(1,3p).
Passing requires adjusted p≤0.05 **and** mean improvement≥0.020 nats. Thus at least
9/10 improvements are necessary. The sign test addresses consistency across
seeds, not specifically the mean. Report every value, mean, median, sample SD,
positive count and p-value; do not treat timesteps as independent replicates.

| Gate | Required condition |
|---|---|
| G0 execution | All ten complete; exact reference/generated CPU/Metal/repeated-Metal trajectories, streams and finite valid probes |
| G1 activity | Every recurrent arm/seed: rate 0.01–0.35 and ≥50% active units in final 10k A and scored B/C; ≤25% endpoint weights after A |
| G2 adaptation | Every Plastic seed: ≥1% weights changed and mean absolute change≥0.10 |
| G3 acquisition | d_F passes statistical and effect criteria |
| G4 specificity | d_Y passes both criteria and all per-sign movement audits pass |
| G5 persistence | Reset, idempotence, weights-only fresh-process B/C replay, restored-initial-weight and disabled-adaptation checks pass |
| G6 temporal information | d_M passes statistical and effect criteria |

Movement audits retain positive requested/realised L1, exact requested-delta
multisets, requested=realised+clipped, ≤0.05 relative realised-movement gap and
≤0.05 difference in clipping fractions. All gates remain required for full success.
G1/G2 failure is a reported outcome, not grounds to replace a seed. G0/G5 failure
invalidates execution, not local learning. G5 verifies implementation semantics;
G3/G4 are already post-reset measurements. Report each gate despite other failures.

## Execution, sealing and failure handling

Use a new workspace importing the original immutable cell/compiler libraries.
The host runner may be copied to add the eligible-tick condition and an explicit
no-reset branch; all such changes are reviewed, tested and sealed before evaluation.
Retain every primary tick hash, per-phase comparison, movement and activity logs,
1,000-tick checkpoints, original probe coefficients, feature arrays and stream
hashes. Three independent seed workers may run concurrently; all phases within a
seed stay ordered. CUDA hardware and cross-platform float64 probe identity are
outside the claim. Lossless archival after validation preserves original hashes.

Commit this protocol before engineering validation. Validate all 700 inherited
fixtures, cadence boundaries, primary/secondary reset selection and the complete
pipeline on seed 3001. Its full A checkpoint must match NE-0b K=4096 exactly.
Disclose validation outcomes without tuning. Then commit an implementation seal
with binary, sources, generated kernels, dependencies, inherited seals, environment
and validation evidence before generating any 4001–4010 stream or network.

All ten evaluation seeds are mandatory, regardless of early results. No tuning,
new probes, new candidates, exclusions, optional stopping, or additional seeds.
Preserve failed attempts. Only exact-config completion or explicitly logged rerun
is allowed for interruption; no silent overwrite. Execution bugs require a
transparent amendment and affected-run invalidation, not scientific retuning.

Positive claim, only if all gates pass: activity-dependent local synaptic
adaptation produced persistent parameter changes that improved held-out temporal
prediction relative to identical frozen dynamics and activity-decorrelated updates.

Otherwise state which conditions failed and the observed effects under this fixed
dynamics, topology, cadence, rule, representation, probe and budget. Do not call a
small detectable effect nonexistent merely because it misses the effect threshold,
or conclude that local learning in general cannot work. Diagnostic continuation
cannot change the primary verdict. Missing or invalid runs supply no confirmatory
learning verdict.
