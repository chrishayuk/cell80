# NE-1: where is temporal information, and what does adaptation change?

**NE-1a measurement specification frozen, 2026-09-17. No NE-1 neural measurements
have been run. NE-1b remains a design draft.**

The authoritative [measurement specification](measurement-spec.md) fixes every
readout, numerical choice, calibration gate, failure policy and output dimension.
Its SHA-256 is recorded in `measurement-spec.sha256` and the
[specification seal](measurement-seal.json). This is a specification freeze;
implementation, calibration and execution seals are still required before runs.

NE-0c produced a valid negative result for its registered learning criteria.
NE-1 investigates that result, using the existing Frozen, Plastic and Yoked
systems. It does not revise NE-0c's probe, gates or conclusion.

NE-0c shows that the viable K=4096 setting still failed the learning criteria.
It does not isolate saturation's contribution to the original K=1 failure:
changing cadence also changed the number and timing of update opportunities.

This is an exploratory mechanism study proposed after viewing NE-0c outcomes.
Reusing seeds 4001–4010 cannot turn it into a fresh confirmation, even with new
measurement streams. Any selected mechanism or architectural remedy needs its
own prospective test on new seeds.

## Questions and limits

1. How much current-symbol and historical information is accessible in state?
2. Does adaptation change that information, or merely its readability by a
   particular probe?
3. Do changes affect prediction beyond changes in activity or amplitude?
4. Which parameter interventions reproduce or reverse those effects?

There is a third possible bottleneck alongside dynamics and learning: the
measurement instrument. Low linear-probe performance is not proof of absent
information. Good history decoding is not proof that a particular next-symbol
probe can compose that history with the current input.

The results may distinguish mechanisms; they cannot establish that a third-factor
rule is necessary, or that the existing rule has no relevant local signal.
Those hypotheses require direct comparisons under new protocols.

## NE-1a: diagnose the representation and instrument first

### 1. Verify the available weight snapshots

The existing archive saves full checkpoints every 1,000 ticks. All new
measurements discard their transient fields, retain weights/topology and reset
state before each independent stream. The desired quarter points have exact
weight equivalents in those archives:

| Requested adaptation time | Archived checkpoint | Updates applied | Why weights are exact |
|---:|---:|---:|---|
| 0 | A-001000 | 0 | First update is at completed tick 4096 |
| 12,500 (25%) | A-013000 | 3 | No update between 12,500 and 13,000 |
| 25,000 (50%) | A-025000 | 6 | Exact saved tick |
| 37,500 (75%) | A-038000 | 9 | No update between 37,500 and 38,000 |
| 50,000 (100%) | A-050000 | 12 | Exact saved tick |

This equivalence applies to persistent weights, **not transient state**. Do not
label the archived neuron state at tick 13,000 as state at tick 12,500.
`inventory.py` verifies source seals, archive hashes, snapshot identities, initial
arm equality and constant Frozen weights. [checkpoint-inventory.json](checkpoint-inventory.json)
records the evidence. The original archives remain untouched.

Recheck from the repository root with:

```sh
python3 experiments/neural-ecology-ne1/inventory.py
```

Keep Yoked throughout: it is still needed to distinguish activity-dependent
adaptation from decorrelated parameter movement. Frozen is a useful identity
control: the same weights, reset and stream must yield identical data at every
adaptation time. At time zero all three arms must also agree exactly. Duplicate
Frozen and zero-time computations can be cached only after identity checks.

### 2. Independent measurement streams

For every saved weight configuration, freeze plasticity, reset all neuron fields,
apply 256 unscored warm-up symbols and collect 10,000 measurement rows. Use the
same symbols across arms and adaptation times within each seed.

Reserve new addressed RNG namespaces `grammar/NE1-v1/fit`,
`grammar/NE1-v1/validation`, and `grammar/NE1-v1/test`, retaining the original
grammar and addressed generator. Each stream starts from its own reset; no state
crosses streams. Generate test data only after all probe choices and fits are
locked. Never use adaptation A as probe-fit data or recycle NE-0c C for tuning.

The validation split may select only a predeclared readout grid, identically
across arms and times. No architecture, plasticity, cadence or encoding search.
Use development seed 3001 for engineering and readout calibration before NE-1
evaluation streams. Its already observed NE-0c outcomes must be disclosed too.

### 3. Precisely indexed targets and input controls

Let h[t] be neuron state **after consuming x[t]**, before x[t+1] is exposed.

| Target | Inputs | Purpose |
|---|---|---|
| x[t] | Neural features alone | Current-symbol readability; never supply the answer as an input |
| x[t-1] | Neural features, with/without current-symbol conditioning | One-step history |
| x[t-2] | Neural features, with/without current-symbol conditioning | Two-step history |
| x[t+1] | Neural features plus current symbol | Predictive usefulness; NE-0c-compatible task |

For lag tasks, report the current-symbol-only baseline on the identical rows.
Always keep targets and features aligned after warm-up; use hand-checked short
sequences to verify indexing. No feature contains future inputs, labels, or an
adaptation-time identifier.

Export both the original trace vector q and complete neuron state (v, r, s, q).
This separates a trace-only representation limit from a full-state limit.
Observe the exact same dynamics; instrumentation must not change trajectories.

### 4. Calibrate the probe before interpreting the reservoir

The grammar is x[t+1]=(x[t-1]+x[t]+z) mod 4, with innovation probabilities
(0.7, 0.1, 0.1, 0.1). The stationary ordered pair is uniform.

Two essential synthetic controls use the **same sampled symbols and splits**:

* **Separate one-hots:** provide the exact previous and current symbols in two
  one-hot blocks to an additive linear-softmax probe. Each block by itself is
  independent of the next symbol in the population. At the uniform predictor,
  every population gradient is zero; convex cross-entropy therefore has an
  optimum at uniform predictions. Perfect history can be present while this
  probe gains nothing. Finite-sample scores can differ from the population value.
* **Joint-pair one-hot:** provide the exact ordered pair as one of sixteen
  categories. This representation supports the true conditional distribution.
  The analytic oracle has expected loss 0.940447989 nats, versus ln(4)=1.386294361
  for the current-symbol reference. A fitted positive-control probe should
  approach the sampled oracle within a predeclared calibration tolerance.

Also verify that lag probes can decode the supplied history one-hots. These
controls test the instrument; they do not count as neural performance.

Retain the exact NE-0c trace scaling, linear probe, L2 and optimization budget as
an anchor. Alongside it, plan a fixed diagnostic comparison:

1. Linear probes on B-normalized q and on B-normalized full state.
2. Current-symbol-conditioned linear probes: four readout heads, one selected by
   the observed current symbol. This permits history/current interactions without
   claiming nonlinear structure must already be linearly decoded by one head.

Use identical fitting/selection procedures and parameter budgets across arms and
times within each probe family. Report the different capacities of the families.
Record feature scales, coefficient norms, objective values and gradient norms.
Scaling changes effective L2 regularization: a scaling benefit alone is not
evidence of newly discovered information or an optimization failure.

The [measurement specification](measurement-spec.md) now fixes normalization,
constant-feature handling, L2 grid, optimization/convergence, validation selection,
ties and oracle-control tolerances. These choices cannot be selected using test
results. A failed positive control makes that instrument unsuitable for
interpreting a negative neural result.

### 5. Refit versus fixed readouts

Independently refit each probe on fit data at each arm/time, then freeze it for
test. This measures how accessible a target is under that instrument.

Separately train a zero-time probe and transport its coefficients and fit-only
normalization unchanged to all later arm/time states. Degradation for the fixed
readout with preserved refitted performance suggests a change in representation;
it does not establish destruction of information. Do not align coordinates using
test labels or choose the most favorable readout after viewing test results.

Report test log loss and descriptive accuracy for every target, view, probe,
arm and time. For each paired contrast retain all ten seed values, mean, median,
sample SD and signs. Timesteps and checkpoints are not additional independent
replicates. This first study is descriptive; no post-hoc p-value gate or selected
checkpoint may be advertised as a new confirmatory learning result.

## NE-1b: relate representation changes to parameter changes

Proceed only with a calibrated instrument and a frozen analysis definition.

On matched input streams, compare state differences h_Plastic[t]-h_Frozen[t]
and h_Yoked[t]-h_Frozen[t], in common neuron coordinates with a common fit-only
normalization. Report activity, state means/variance, clipping, and weight-change
statistics alongside decoding. Large movement is not itself task alignment.

Define predictive directions from fit-only readout coefficients restricted to
neural coordinates, and nuisance directions from current-symbol decoding,
population activity and amplitude. Report shared and unique projection energy;
the spans can overlap. Freeze rank cutoffs and handle zero-rank/zero-change cases.
Measure held-out loss changes as well: projection magnitude alone has no sign
and can correspond to movement in the wrong predictive direction.

Weight deltas and neural features inhabit different spaces. Do not take their
dot product and call it alignment. Connecting an edge change to a predictive
direction requires a defined perturbation/response measurement.

For causal follow-up, reset and replay the same input stream with:

* the learned weights unchanged (control);
* the original weights restored (exact Frozen recovery control);
* a prespecified subset of changed edges reverted;
* matched random reversion controls preserving source-sign and movement budgets.

Choose intervention subsets and matching rules on fit/validation data only.
Specify intervention sizes, random addresses, matching tolerances, readout policy
(frozen versus refitted), costs and contrasts **before** generating intervention
test outcomes. These details are deliberately not frozen by this design draft.
Small finite changes, rather than gradients through a thresholded integer model,
should define edge effects. This optional stage must not delay or silently alter
the complete NE-1a matrix.

## How to interpret the patterns

| Pattern | Supported interpretation | What remains unresolved |
|---|---|---|
| Synthetic known-history control fails | Instrument cannot express/fit the known solution | Neural information content |
| Conditioned readout succeeds, original readout fails | Task interaction/readout restriction matters | Whether plasticity improved the representation |
| Full-state readout exceeds q-only readout | Information is accessible outside the original trace features | Whether it is useful to an endogenous downstream circuit |
| Fixed readout worsens; refitted readout recovers | Representation changed relative to the fixed reader | Total information loss |
| Plastic worsens calibrated history/next-symbol decoding versus Frozen and Yoked | Rule impairs accessibility under tested instruments | Destruction of information for all possible readers |
| Neither arm supports history decoding with calibrated probes | Dynamics/encoding or an untested decoding limit is a candidate bottleneck | Absence of historical information |
| Calibrated Frozen approaches sampled oracle | Little room for predictive improvement under that instrument | Learning on other tasks or distributions |
| Plastic mainly changes activity; task readouts do not improve | Adaptation is not aligned with this measured task | Necessity or superiority of reward/error modulation |

At present Frozen is far from the full-history oracle under NE-0c's probe.
“The reservoir already contains nearly all available predictive information” is
a hypothesis to test with calibrated instruments, not the current finding.

## Execution boundary and artifacts

Implement instrumentation in a new workspace. Preserve all NE-0/NE-0b/NE-0c
sealed sources, binaries, reports and archives. No new A adaptation is needed for
NE-1a. New trajectories are frozen-weight measurement branches only.

Before those runs, commit the numerical measurement specification and code seal.
Validate synthetic positive controls, label indexing, zero-time identity, repeated
Frozen identity, no weight updates, deterministic replay and instrumentation
parity on the development seed. Keep failure records and report all planned
cells of the matrix, including bad or uninformative results.

Expected outputs: checkpoint inventory; locked measurement specification;
per-target/per-probe per-seed loss tables across all five times; feature/activity
diagnostics; calibration results; provenance and replay audit. NE-1b requires its
own intervention specification. A mechanistic claim selected from these existing
seeds requires fresh-seed replication before confirmation.
