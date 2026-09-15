# NE-0 — persistent local adaptation and temporal prediction

**Protocol v1.0 | 2026-09-15 | specified before neural implementation or runs.**

This file fixes the scientific protocol. Its SHA-256 is recorded in the adjacent
`neural-ecology-ne0-preregistration.sha256`. This is a local, hash-sealed protocol,
not an externally timestamped registration. No execution, activity, learning, or
backend-equivalence result is asserted. Before evaluation, an implementation seal
must additionally identify the source revision, executable cells, dependencies,
and fixtures (§12). The initial numerical choices below are experimental choices,
not previously validated operating parameters.

## 1. Question and scope

Does activity-dependent local synaptic adaptation produce persistent changes that
improve held-out next-symbol prediction relative to identical frozen dynamics and
activity-decorrelated weight updates?

The causal chain under test is adaptation experience → local updates → persistent
weights → information decodable after transient-state reset. A fitted readout is
a measurement instrument; neither its supervised fitting nor reservoir activity
alone counts as local acquisition. There is one topology family, one neuron rule,
one plasticity rule, and one grammar. No architecture search is part of evaluation.

Cell80's existing `CellGraph` is an acyclic, host-routed composition mechanism.
This experiment will instead use a host tick scheduler, private stateful cells,
and deterministic host edge gathering. It does not require modifying `CellGraph`.
Node dynamics and edge plasticity must execute as Cell80 cells; the host may
generate inputs, route messages, reduce currents, apply weight bounds, and fit
probes. A host-only mathematical simulation does not establish the Cell80 claim.

## 2. Fixed design

| Quantity | Registered value |
|---|---|
| Units | 1,024; IDs 0–1,023 |
| Recurrent edges | Exactly 16 incoming edges per unit; 16,384 total |
| Topology | Fixed; no self edges or duplicate source/target pairs |
| Source signs | IDs 0–767 excitatory (+1); 768–1,023 inhibitory (−1) |
| Weight magnitudes | Integers 0–32, initially uniform on 1–16 |
| Adaptation A | 50,000 ticks; one symbol per tick |
| Probe fitting B | 10,000 scored examples after 256 warm-up ticks |
| Test C | 10,000 scored examples after 256 warm-up ticks |
| Evaluation replicates | 10 paired seeds: 2001–2010 inclusive |
| Development replicates | Seeds 1001, 1002, 1003 only |
| Arms | Frozen, Plastic, Yoked, Memoryless |
| Primary unit | One seed's mean C log loss, in natural-log units (nats) |
| Minimum mean improvement | 0.020 nats for each registered contrast |

The three recurrent arms share topology, initial weights, initial transient state,
and A/B/C symbol streams within a seed. Different seeds independently generate
these quantities. Memoryless receives the same current symbols and targets.

## 3. Exact temporal grammar and references

Symbols are integers 0, 1, 2, 3. For every independent stream, draw the initial
ordered pair uniformly from the 16 possible pairs. Subsequently:

```text
z = 0 with probability 7/10
z = 1, 2, or 3, each with probability 1/10
x[t+1] = (x[t-1] + x[t] + z) mod 4
```

Draw a uniform integer r in 0–9: r < 7 gives z=0, otherwise z=r−6. This defines
all 16 history contexts and all four possible next symbols, without unspecified
transitions or zero-probability targets.

The uniform distribution on ordered pairs is stationary: for every destination
pair (b,c), summing P(c|a,b) over a gives 1. Consequently P(next|current)=1/4,
whereas the correct two-symbol history gives probabilities {0.7,0.1,0.1,0.1}.

| Reference | Population expected log loss | Expected accuracy |
|---|---:|---:|
| Current-symbol Bayes predictor | ln(4) = 1.3862943611198906 | 0.25 |
| Two-symbol-history Bayes predictor | −0.7 ln(0.7) −0.3 ln(0.1) = 0.9404479886553263 | 0.70 |

These are expected **loss floors**, not accuracy ceilings expressed as losses.
The available conditional information is 0.4458463724645644 nats. Also score the
two-symbol oracle on the actual C targets; its finite-sample loss need not equal
the population entropy. The uniform predictor's empirical loss is exactly ln(4).

A, B, and C use independent random namespaces and fresh starting pairs. Entire
streams are new instances; shared short subsequences are legitimate draws from
the same grammar, not grounds for rejection or resampling. This experiment tests
new instances under unchanged statistics, not transfer to a new grammar.

At tick t, only x[t] enters the network. The probe feature is extracted after that
tick's node update, and its target is x[t+1]. Neither that target nor x[t−1] is
supplied directly to the network, plasticity rule, or ordinary probe.

## 4. Randomness, topology and canonical ordering

Use counter-addressed SHA-256 randomness, not a shared advancing RNG. For each
draw hash UTF-8 `NE0-v1/{seed}/{namespace}/{counter}/{attempt}` with decimal
integers, no leading zeros or trailing newline. Interpret the first eight digest
bytes as unsigned little-endian u64. A bounded draw in [0,m) rejects values
at least `floor(2^64/m)*m`; increment attempt from zero until accepted, then
return the value modulo m. Counter starts at zero independently per namespace.

Namespaces are `grammar/A`, `grammar/B`, `grammar/C`, `topology/{target}`,
`weight/{edge_id}`, `yoke/{sign}/{candidate}`, and
`shuffle/{arm}/{current_symbol}`. Each grammar uses its first two draws for the
initial pair (m=4) and subsequent draws for innovations (m=10). Pre-generating a
stream versus requesting its symbols lazily must yield identical bytes.

For each target, start with ascending source IDs excluding that target; perform
descending Fisher–Yates (i from length−1 through 1; draw j in [0,i]); select the
first 16 entries and sort them. Canonical edge order is (target,source), with
edge IDs assigned consecutively from zero. Each initial magnitude is the first
bounded draw for its edge (m=16), plus 1. Source signs never change.

All reductions use ascending canonical edge order. No floating-point atomic
accumulation is permitted. Neural arithmetic uses checked signed i64
intermediates and explicit division/rounding; serialized state uses fixed-width
little-endian fields. Overflow is an execution failure, never implicit wrapping.

## 5. Persistent versus transient state

| Class | Fields and reset semantics |
|---|---|
| Immutable configuration | Topology, source signs, sensory mapping, threshold 64, rule constants |
| Persistent learned parameters | One u16 weight magnitude per edge |
| Node transient state | u16 membrane v, u8 refractory r, u8 last spike s, u16 activity trace q |
| Edge transient state | None; plasticity reads node traces from the tick-start snapshot |
| Scheduler transient state | Phase counters, current buffers, pending deltas; reset to empty/zero |
| Measurement state | Probe coefficients and features; separate from neural state |

Valid node ranges are v∈[0,255], r∈{0,1}, s∈{0,1}, q∈[0,255]. Initial and reset
node state is all zeros. Thresholds are fixed, not learnable. There are no hidden
delay buffers, eligibility traces, adaptive thresholds, or stochastic neuron
states. Adding any of these requires a new protocol version.

`reset_transients()` preserves persistent and immutable bytes exactly, zeros all
transient state, and is idempotent. A serialized persistent checkpoint plus the
immutable configuration must reconstruct the same post-reset network in a fresh
process. B and C each start from this reconstruction, not from A's or B's final
activity. Neither checkpoint contains a symbol history.

## 6. One tick: exact dynamics and plasticity

All node/edge reads below use the same tick-start snapshot except explicitly
named new node outputs. An old spike propagates with a one-tick delay.

```text
I[i] = sum over edges j→i of sign[j] * w[j→i] * s_old[j]
       + (48 if i mod 4 == x[t], else 0)

if r_old[i] == 1:
    v_new[i] = 0; s_new[i] = 0; r_new[i] = 0
else:
    u = clamp(floor(7*v_old[i]/8) + I[i], 0, 255)
    s_new[i] = 1 if u >= 64 else 0
    v_new[i] = 0 if s_new[i] == 1 else u
    r_new[i] = s_new[i]

q_new[i] = min(255, floor(15*q_old[i]/16) + 64*s_new[i])

For Plastic edge j→i during A only:
    a = q_old[j]*s_new[i] - q_old[i]*s_old[j]
    delta[j→i] = clamp(trunc_toward_zero(a/64), -1, 1)
    w_new[j→i] = clamp(w_old[j→i] + delta[j→i], 0, 32)
```

This is the single registered timing-dependent local rule. It is a chosen
computational rule, not a claim of biological fidelity or a guarantee of useful
learning. Both inhibitory and excitatory edges update magnitudes with the same
rule; immutable source signs determine their current contribution.

Phases are snapshot → old-weight current reduction → node update → requested
plasticity → bounded weight update → commit. No new weight affects current in
the same tick. Frozen always requests zero deltas. After A, all arms request zero
deltas, including during warm-up, B, C, and diagnostic replays. Log traces and
currents independently of whether plasticity is enabled.

## 7. Yoked arm and movement audit

Before A, partition edges by immutable source sign. Independently within each
partition, use Fisher–Yates on ascending edge IDs to obtain a permutation; reject
any candidate with a fixed point and try the next candidate namespace. Candidate
starts at zero. Store the accepted permutations before adaptation. There is one
fixed permutation per seed and sign for the whole experiment.

For destination edge e, Yoked requests Plastic's `delta[pi(e)]` from the same
tick, then applies its own [0,32] bounds. It does not compute a delta from its own
activity. Signs, initial magnitudes and topology remain as originally paired;
initial weights are not permuted. This is an activity-decorrelated control by
construction, not a mathematical guarantee of zero incidental correlation.

For each tick and sign, and for the complete A run, record:

```text
requested_L1 = sum(abs(requested_delta))
realised_L1  = sum(abs(w_new - w_old))
clipped_L1   = sum(abs(requested_delta - (w_new - w_old)))
```

Also record nonzero-update counts, net signed change, endpoint fraction, and
the distribution of realised deltas. Requested delta multisets must match exactly
between Plastic and Yoked within each sign/tick. In this scalar bounded update,
requested_L1 = realised_L1 + clipped_L1 must hold exactly.

For specificity to be interpretable, in every evaluation seed and sign:

- both arms have positive requested_L1 and realised_L1;
- `abs(realised_P-realised_Y)/max(realised_P,realised_Y) <= 0.05`;
- the difference in `clipped_L1/requested_L1` between arms is at most 0.05.

These audit thresholds are fixed validity conditions for G4, not reasons to tune
or select seeds. Even a passing audit matches only these measured aspects of
movement; it cannot exclude every possible change in reservoir dynamics.

## 8. Measurement protocol

After adaptation, preserve each recurrent arm's weights. For B and separately C:
reset all transients; ingest 256 unscored symbols; collect features for the next
10,000 symbols and retain one additional symbol solely as the last target.
The stream starts with its registered initial pair; the network ingests x[0]
on its first warm-up tick. B never supplies a warm start to C.

The recurrent feature vector has 1,028 entries: `q_new[i]/(255*32)` for every
node in ID order, followed by a four-way one-hot encoding of x[t]. The factor
32 is sqrt(1,024); no data-dependent standardization or feature selection occurs.
The memoryless feature has its first 1,024 entries fixed to zero, with the same
current-symbol one-hot suffix. The uniform Bayes reference remains the exact
memoryless information limit; equality of unused probe dimensions is not a claim
of equal functional capacity.

Fit a separate four-class linear softmax probe for every arm and seed, on B only.
Use an intercept and minimize mean cross-entropy plus
`0.01/2 * sum(W*W)`, with the intercept unpenalized. Initialize all coefficients
and intercepts to zero. Use exactly 2,000 full-batch gradient-descent steps of
size 0.5, float64 arithmetic, stable log-sum-exp, no momentum, minibatching,
early stopping, class reweighting, validation search, or alternative fits.
This defines a fixed-budget instrument, not an asserted optimum. Report initial
and final objective and final gradient norm. Nonfinite values or a final objective
above the initial objective by more than 1e-10 invalidate the measurement.

Store B-feature hashes and the fitted coefficients before generating/scoring C.
Never fit, calibrate, select, or normalize using C. Evaluate C with the frozen
probe, using stable log-softmax without arbitrary probability clipping. Accuracy
is secondary; ties select the lowest symbol ID. Report oracle C loss as well.

The probe runs on the CPU in a recorded float64 environment with numerical-library
thread count 1. Exact neural replay is required across CPU/Metal; cross-platform
bit identity of the floating-point probe is not claimed. Use one canonical probe
run for identical neural feature arrays, and retain coefficient/output hashes.

### Conditional shuffled-state diagnostic (secondary; no gate)

For each recurrent arm and seed, take its normal C examples and partition by
current symbol. Independently shuffle entire neural feature vectors within each
partition using the registered shuffle namespace; retain each row's current
symbol, target, and the already fitted probe. Do not refit. Thus the current-symbol
association and conditional neural-feature marginals remain intact, while
alignment with the actual preceding history is disrupted. Use one shuffle only.

Report normal versus shuffled loss per seed. This is a descriptive alignment
check, not an exact permutation significance test for correlated time-series
rows. A global shuffle would also destroy current-symbol information and is
therefore not the registered diagnostic.

## 9. Statistical analysis and decision rules

Let L[a,s] be arm a's mean C loss for seed s. Define:

```text
d_F[s] = L[Frozen,s] - L[Plastic,s]
d_Y[s] = L[Yoked,s] - L[Plastic,s]
d_M[s] = ln(4) - L[Plastic,s]
```

Positive means improvement. Report all ten values, means, medians, sample standard
deviations, and counts strictly above zero. Do not treat individual ticks as
independent replicates. No seed exclusions, replacements, optional stopping, or
additional evaluation seeds are allowed.

For each of the three contrasts, use the one-sided exact paired sign test of the
null that a seed improves with probability at most 1/2. With k strictly positive
differences, compute `sum(comb(10,j), j=k..10)/1024`; zero counts as nonpositive.
This tests consistency across seeds, not a null specifically about the mean.
Use Bonferroni alpha=0.05/3 for the family of three tests. With n=10, significance
requires at least 9 positive seeds (p=0.0107421875); 8 do not suffice. Report both
raw p and `min(1,3*p)`; compare the latter to 0.05.

Each contrast passes only if its adjusted test passes **and** its mean improvement
is at least 0.020 nats. The mean threshold is a separate minimum effect condition,
not a claim of a confidence bound above 0.020. This small study has limited power;
no power calculation or expected effect is asserted. Report all tests, including
failed tests. No other inferential test may replace these after results are seen.

## 10. G0–G6 and interpretation

| Gate | Pass condition | Failure interpretation |
|---|---|---|
| G0 execution | Exact CPU-reference/Cell80-CPU/Metal state agreement, same-backend replay, prescribed streams, no arithmetic or probe failures, and full evaluation completion (§11) | Execution or measurement invalid/incomplete; no learning verdict |
| G1 activity | For every recurrent arm and seed, in the last 10,000 A ticks and in scored B and C, mean spike fraction is in [0.01,0.35] and at least 50% of units spike at least once; final A weight endpoint fraction is at most 25% | Registered operating regime failed; retain all rows, do not select a viable subset |
| G2 adaptation | In every Plastic seed, at least 1% of weights differ from initialization and mean absolute final-minus-initial weight change is at least 0.10 integer units | Persistent movement below registered minimum |
| G3 acquisition | d_F passes both statistical and effect criteria | No registered improvement over frozen dynamics |
| G4 specificity | d_Y passes both criteria, and all movement audits in §7 pass | Either specificity unsupported or movement mismatch confounds interpretation; distinguish these cases |
| G5 persistence | Reset/idempotence/checkpoint tests pass and fresh-process reconstruction plus reset exactly reproduces all primary post-reset B/C neural features; pre-learning-weight restoration reproduces Frozen (§11) | Persistence implementation invalid; cannot attribute primary result to stored weights |
| G6 temporal information | d_M passes both criteria | No registered evidence of exceeding the current-symbol information limit |

The full positive claim requires **all gates**. Evaluate and publish every gate
for completed data even when another fails. G1/G2 failures are outcomes for this
configuration, not bugs by definition. G0/G5 failures are validity failures, not
evidence against local learning. Missing runs yield INCOMPLETE, never a fabricated
zero loss or a reduced-n positive result.

G3/G4 are already measured after transient reset. G5 therefore validates that
reset/checkpoint semantics were actually honored; it is not an independent
statistical comparison of reset versus no reset. No-reset performance cannot
rescue a primary failure. An optional later no-reset study must be explicitly
exploratory and cannot be described as a registered NE-0 gate.

Examples of bounded readings:

- G3 pass, G4 contrast fail: adaptation helps relative to Frozen, but the
  activity-dependent rule has not outperformed this yoked control.
- G4 audit fail: insufficient control of realised movement for the specificity
  claim, even if Plastic has lower loss.
- G3/G4 pass, G6 fail: better decodability without demonstrated prediction beyond
  the current-symbol reference.
- All gates pass: persistent adaptation improved temporal prediction under this
  particular rule, topology, grammar, probe and budget. This does not establish a
  mechanism beyond these controls, biological fidelity, or general intelligence.

## 11. Execution and causal integrity checks

Before evaluation, implement an independent host reference directly from §6,
separately from the executable node and edge cells. Verify manually checkable
fixtures: refractory suppression, positive/negative current, threshold equality,
trace saturation, signed truncation, both weight bounds, and old/new phase order.
Check canonical packing and output bounds, not only final prediction accuracy.

On development seeds, compare all fields after every phase for the first 256
ticks across reference, Cell80 CPU, and Metal. For every full evaluation seed and
recurrent arm, run all three backends and repeat Metal once. Require byte-identical
state hashes at every committed tick through A, B, and C, including topology,
weights, node state, phase index, input and requested deltas; exclude backend
identifiers/timings. Retain full checkpoints every 1,000 ticks and at phase
boundaries to localize any mismatch. Hashing uses canonical bytes and SHA-256.
Timing, throughput and peak memory are descriptive, with host and probe costs
included separately. CUDA is outside this registration.

Required reset/intervention checks for every evaluation seed:

1. Hash immutable/persistent state before and after reset; it must be unchanged.
   Transient bytes must equal the canonical zero state. Reset twice equals once.
2. Reconstruct from each final-A persistent checkpoint in a fresh process; reset
   and replay B/C. Neural feature bytes must equal the primary execution exactly.
   Reuse the stored probe rather than fitting a favorable replacement.
3. Replace Plastic's final weights with the initial weights, reset, and replay
   B/C. Full neural states/features must equal Frozen exactly. Score with Frozen's
   probe for the identity check; using Plastic's probe is a different intervention.
4. Disable Plastic's update application from the initial snapshot on A. It must
   reproduce Frozen, irrespective of whether requested deltas are computed for
   diagnostics; compare applied state, not intentionally different delta logs.
5. Assert every B/C weight hash is constant and all applied deltas are zero.
   Assert Yoked receives the saved permutation of Plastic requests, not realised
   changes or its own activity-dependent requests.
6. The probe-data exporter must emit only registered features and targets; test
   off-by-one alignment against hand-written short streams and forbid future
   symbols, row IDs, seed IDs, or grammar context in features.

## 12. Development, sealing and amendments

Development seeds may be inspected for activity collapse, saturation, clipping,
overflow, probe behavior, grammar correctness and runtime cost. They never count
toward the ten evaluation replicates. No evaluation A/B/C streams or trajectories
may be generated or inspected during development. The theoretical grammar
calculation in §3 does not consume development or evaluation data.

The v1.0 numerical choices are fixed now. If development indicates a change is
needed, retain v1.0 and publish a separately named, hashed amendment with the
reason and all examined development outcomes **before** accessing evaluation
data. Never silently replace this protocol or call a tuned configuration v1.0.
After evaluation access, changes require a new experiment and new evaluation
seed family fixed in advance; original results remain reportable.

Before evaluation, write an implementation-seal manifest with this protocol hash,
source commit and any dirty-source patch hashes, dependency lock hashes, CPU and
Metal environment versions, canonical config, cell source/bytecode hashes,
fixture outputs, development-run inventory, and declared evaluation seeds. Seal
the manifest in Git before running evaluation. A timestamp/hash records what was
fixed; it is not evidence of successful execution. No hashes for future code or
unrun fixtures are invented in this document.

All ten seeds are mandatory, even if early losses look poor. A hardware interruption
may resume an exact recorded checkpoint or rerun the identical seed/config; log
both attempts. Execution bugs must be reported and fixed transparently with the
affected outputs invalidated. They must not be used as an opportunity to change
scientific parameters or replace difficult seeds.

## 13. Artifacts and report

Keep protocol, seal manifests, implementation, verification fixtures and compact
findings in Git. Put bulky output under `experiments/neural-ecology/artifacts/`,
following the experiment repository convention:

- `datasets/`: A/B/C stream hashes and generated streams, permutation maps,
  feature arrays, target arrays, shuffled-row maps;
- `checkpoints/`: initial/final weights, periodic state checkpoints, frozen probes;
- `logs/`: gate checks, environment details, timing/memory, per-tick hashes,
  per-sign movement statistics, failed/resumed attempts;
- a compact per-seed results table with all arm losses, accuracies, oracle loss,
  d_F/d_Y/d_M, shuffled diagnostic, activity, clipping, and gate outcomes.

The report must state protocol/seal hashes, completeness, all ten paired results,
three exact p-values, effect summaries, and G0–G6 separately. No illustrative
accuracy numbers in the preceding discussion are predictions or results.

**Positive wording, only if every gate passes:** Under the registered conditions,
activity-dependent local synaptic adaptation produced persistent parameter changes
that improved held-out temporal prediction relative to identical frozen dynamics
and the registered activity-decorrelated update control.

**Valid negative wording:** Under the registered dynamics, topology, local rule,
representation, probe and adaptation budget, the experiment did not meet the
specified criteria for persistent, activity-specific temporal learning. Name the
failed gates and observed effects; do not infer that local learning cannot work.

**Invalid or incomplete wording:** Execution, reset, measurement, or completion
requirements were not satisfied; NE-0 supplies no confirmatory learning verdict.
