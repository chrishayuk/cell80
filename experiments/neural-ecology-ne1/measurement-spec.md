# NE-1a numerical measurement specification

**Version 1.0 — 2026-09-17. Frozen before any NE-1 neural measurement.**

This specification overrides unresolved measurement choices in the [design](README.md).
NE-1a is exploratory on previously observed seeds; it cannot revise NE-0c's verdict.
The separate specification seal records file hashes, not a completed implementation
or a passed calibration. No NE-1b intervention may begin before the entire NE-1a
matrix, including failure records, is complete and audited. NE-1b then needs its
own frozen intervention specification.

## 1. Systems, streams and indexing

Evaluation seeds: **4001–4010**, all mandatory. Development: **3001**, separately
reported, never pooled. Arms: **Frozen, Plastic, Yoked**. Requested adaptation
ticks: **0, 12500, 25000, 37500, 50000**, sourced from archived ticks
**1000, 13000, 25000, 38000, 50000** respectively. Only weights/topology are retained;
all v/r/s/q fields reset to zero independently for every measurement stream.
Verify the inventory, no intervening eligible weight update, initial arm identity,
and unchanged Frozen weights. No new adaptation, different cadence, or edge changes.

Use the original `NE0-v1/{seed}/...` addressed generator and grammar, with exact
namespaces `grammar/NE1-v1/fit`, `grammar/NE1-v1/validation`,
`grammar/NE1-v1/test`. Each stream contains **10257 symbols**, x[0]…x[10256].
Consume x[0]…x[10255]; export **10000** rows after consuming x[t], for
**t=256…10255** inclusive. The first 256 ticks are unscored warm-up.
Targets are current=x[t], lag1=x[t−1], lag2=x[t−2], next=x[t+1].
Plasticity is disabled throughout; assert every requested delta is zero and
persistent weight hashes are unchanged. Each seed uses identical streams across
all arms/times. No state crosses streams. Labels never enter neural computation.

Generate evaluation fit/validation streams first. Freeze every evaluation probe,
normalizer and selected lambda, across all ten seeds, in a hashed fit manifest
**before generating any evaluation test symbols or trajectories**. Synthetic
controls obey that same barrier. Development has its own barrier and artifacts.

## 2. Features and normalization

Node order is ascending ID. Convert integers exactly to float64.

* `q`: `[q0,…,q1023]`, dimension **1024**.
* `full`: `[v0,…,v1023,r0,…,r1023,s0,…,s1023,q0,…,q1023]`, dimension **4096**.
  Keep both r and s even if duplicate; no feature selection, PCA or decorrelation.

For each arm/time/view, on **all 10000 fit rows only**, compute
`mu[j]=mean(x[:,j])`, `var[j]=mean((x[:,j]-mu[j])**2)` (ddof=0), and
`sigma[j]=sqrt(var[j])`. If `sigma[j] <= 1e-12`, mark the coordinate constant and
set its transformed value to zero on **every** split, including transported tests.
Otherwise use `(x[j]-mu[j])/sigma[j]`. Retain the column; do not drop it.
No clipping, per-row normalization or further dimension scaling. Record all
means, scales and masks. Reuse the same normalizer across targets and head types
for that arm/time/view. Four-head probes use the whole-fit normalizer, not four
separately normalized subsets. Current-symbol one-hots, where specified below,
are appended as raw 0/1 values and are never centered/scaled.

## 3. Exact readout matrix

All classifiers output four softmax probabilities. `G` is a single global head;
`C` routes a row to head x[t] in a set of four independent heads.

| View | Family | Targets | Neural inputs | Current-symbol access | Cells per system |
|---|---|---|---|---|---:|
| q, full | G | current, lag1, lag2 | normalized view | none | 6 |
| q, full | G | next | normalized view | appended 4-way one-hot | 2 |
| q, full | C | lag1, lag2, next | normalized view | routing only | 6 |
| q | anchor | next | q/(255×32), no centering | appended 4-way one-hot | 1 |

Thus **15 readout cells**, with no conditioned-current cell that would expose its
own answer. Each cell has **refitted** and **transported** test rows.
Global normalized parameter count is `4d+4` (history/current), or `4(d+4)+4`
(next). Conditioned count is `4*(4d+4)`. Anchor count is `4*1028+4`.
Report these capacity differences; comparisons between families are diagnostic.

Refitted: fit and select independently per arm/time/readout. Transported: use the
**Frozen time-zero** selected coefficients, lambda, intercepts, constant mask and
normalizer unchanged for that seed/view/readout at every arm/time. Do not refit,
recenter or recalibrate transported models. Zero-time arms must agree exactly;
transported and refitted zero-time predictions must agree in the recorded numeric
environment. Frozen predictions must be identical across all five times.

## 4. Objective, solver and selection

Normalized G: minimize `mean(-log p[y]) + (lambda/2)*sum(W**2)`.
Normalized C: fit four separate objectives of this form, each averaged over its
own fit subset (not the full dataset). A **single shared lambda** is selected for
the quartet using row-weighted pooled validation loss. No head-specific selection.
L2 includes every coefficient, including appended one-hot coefficients; **no
intercept is regularized**. No class weights, label smoothing or loss clipping.

**Lambda grid, in fit order:** `[0.0001, 0.001, 0.01, 0.1, 1.0]`.
Start every candidate/head independently at all-zero coefficients/intercepts.
No warm-starting between lambdas, arms, times or targets. Stable log-softmax
subtracts each row's maximum logit before exponentiation; use analytic gradients.

Use Python 3.12, **NumPy 2.4.2, SciPy 1.17.0, threadpoolctl 3.6.0**, float64 and
one BLAS/OpenMP thread. Pin exact Python patch, libraries, build and hardware in
the execution seal. Call `scipy.optimize.minimize`, method `L-BFGS-B`, `jac=True`,
`bounds=None`, options **maxiter=2000, maxfun=20000, maxcor=20, maxls=50,
gtol=1e-6, ftol=0.0**. Flatten W in C order followed by b. No numerical gradients,
stochastic minibatches or optimization randomness.

A candidate is valid only if the solver reports success, all outputs are finite,
the independently recomputed gradient infinity norm is **<=1e-6**, and the final
penalized objective is no more than its initial value plus **1e-10**. Every C head
must pass. Relative-function-change termination alone does not satisfy the
gradient criterion. Retain status/message, iterations/evaluations, objective,
gradient norm and coefficient norms for every candidate/head.

Select by **unpenalized mean validation log loss**. Let m be the exact minimum
among five candidate losses; all losses `<=m+1e-8` are tied. Choose the **largest
lambda** in that set. Do not refit on fit+validation. Accuracy never selects a
model. Each current-symbol group must have at least **128 rows** in each split;
otherwise record a stream/calibration failure, with no redraw or seed replacement.

The anchor is exempt from new solver/normalization choices: use NE-0c's exact
four-class linear probe, zero initialization, L2=0.01 on W only, unpenalized b,
**2000 full-batch gradient steps, learning rate 0.5**, and its original finite/
nonincreasing-objective check. Record its gradient but impose no new convergence
gate. It has no lambda selection and is not required to pass expressive-readout
positive controls. This preserves the original instrument as a comparison.

## 5. Baselines and synthetic-control gates

Report uniform loss ln(4)=**1.3862943611198906** for all four targets. For next,
also report the sampled oracle using probabilities 0.7 for
`(x[t−1]+x[t]) mod 4`, 0.1 otherwise; population expected loss is
**0.9404479886553263**. Do not compare finite-sample scores only to an expected
entropy. These references need no fitted parameters.

Four fitted baseline rows per seed: current uses fit target frequencies only;
lag1, lag2 and next use fit target frequencies conditional on current symbol.
Use add-one smoothing: `(count_y+1)/(group_count+4)`, with a single global group
for current. No validation selection. Baselines are shared across arms/times.

Synthetic controls use the exact same splits/targets and solver/grid/selection
as their normalized neural family. Zero-pad neural coordinates to **each of
1024 and 4096 dimensions**, then apply the same fit-only normalization. For one-
hot controls use the lowest coordinates; unused coordinates are constant zeros.

| Family | Synthetic neural input | Targets | Required test gate |
|---|---|---|---|
| G | one-hot of the target | current, lag1, lag2 | each accuracy >=0.999 and loss <=0.010 |
| G | 16-way one-hot of `4*x[t−1]+x[t]` | next; current one-hot appended | loss minus sampled oracle <=0.010 and accuracy >=0.65 |
| G | one-hot x[t−1] | next; current one-hot appended | absolute difference from ln(4) <=0.010 |
| C | one-hot of the target | lag1, lag2 | each accuracy >=0.999 and loss <=0.010 |
| C | one-hot x[t−1] | next; routed by current | loss minus sampled oracle <=0.010 and accuracy >=0.65 |

That is **8 controls per width, 16 per seed**. The copying controls deliberately
provide labels as synthetic inputs; they validate the instrument and never count
as neural decoding. The additive next control is an expected limitation, not an
expressive positive control. Also enumerate the exact 16 histories ×4 innovations
to check uniform individual-symbol/target marginals and zero population gradient
for separate one-hots (absolute tolerance **1e-14**).

All development controls must pass before evaluation neural measurement begins.
Evaluation controls are fitted before the global test barrier and scored on each
seed's test stream. A failed control marks the linked family/target/width invalid
for mechanistic interpretation, but its score and all planned neural rows remain
reported. Finish the matrix; do not suppress a difficult seed or retune a reader.

## 6. Failure, parity and outputs

**No automatic solver retry or fallback.** If any lambda candidate fails, record
the entire refitted cell as `invalid_fit` with null selected score; do not select
from survivors. Continue other planned fits. Invalid zero-time selection also
invalidates that transported cell at every later arm/time. Anchor failures use
the same explicit invalid-row policy. Never substitute chance scores for failures.

Neural parity/reset/weight-integrity failures invalidate the affected trajectory
and its dependent cells. Preserve attempts. External interruptions permit only
logged exact-configuration reruns with retained partial artifacts. A numerical or
scientific change requires a new version, amendment and hash; never overwrite v1.
There is no optional stopping, arm omission or seed replacement.

Before evaluation, on development seed 3001 validate label indexing with a short
hand-checked fixture; run the full measurement matrix; check all zero-time and
Frozen identities; and compare reference/generated CPU/Metal/repeated Metal at
every measurement tick. Use those same four-way comparisons during evaluation.
Verify saved/reloaded weights-only checkpoints and reproduce probe predictions
from saved arrays/coefficients. Float64 identity is claimed only in the sealed
numerical environment, not across hardware or library versions.

Evaluation output dimensions (invalid cells still occupy a row):

* **4500 neural test rows** = 10 seeds ×3 arms ×5 times ×15 readouts ×2 modes.
* **10650 logical candidate rows** = 150 systems ×(14 readouts ×5 lambdas +1 anchor),
  with per-head optimizer details nested inside each candidate. Transported models
  have references to their zero-time fit records, not new candidate fits.
* **40 fitted-baseline rows** =10 seeds ×4 targets, plus sampled analytic references.
* **160 selected calibration rows** and **800 calibration candidate rows** on the
  evaluation cohort. Development contributes **450 neural, 1065 candidate,
  4 baseline, 16 calibration and 80 calibration-candidate rows**, kept separate.
* **3000 paired contrast rows** =10 seeds ×5 times ×15 readouts ×2 modes ×2 controls
  (Frozen−Plastic and Yoked−Plastic log loss); null if either score is invalid.

Every score row includes seed/cohort, requested/archive tick, arm, view, target,
family, mode, status, sample count, loss, accuracy, lambda, normalization/model/
stream/feature hashes and calibration status. Retain all candidate validation
scores and test probability hashes. Cache exact duplicate fits/trajectories only
after identity validation; preserve all logical rows and provenance references.

Report complete per-seed tables and paired means, medians, sample SDs and positive
counts for each time/readout/mode. Do not aggregate incomplete contrasts over a
favorable subset: report valid counts and mark the summary incomplete. The seed
is the unit; no pooled-timestep inferential tests, p-value gates or best-checkpoint
selection. No single ranking over different targets/families. Record activity,
feature moments, constant masks and weight movement for every arm/time as context.

## 7. Freeze and execution sequence

1. Commit this specification, design, inventory, dependency pins and specification
   seal before any NE-1 neural run. The seal includes source NE-0c provenance.
2. Implement the separate measurement runner and probes without altering sealed
   NE-0c files. Hash code/build/dependencies before development neural validation.
   Validate synthetic controls first; record every failure and bug-fix revision.
3. Validate the development matrix unchanged; seal the successful implementation,
   environment and validation evidence before evaluation fit/validation streams.
4. Complete all evaluation fitting and write the frozen fit manifest; generate
   test streams, finish the full matrix and mechanically audit it.
5. Only then interpret NE-1a and specify any NE-1b intervention separately.

No seal in step 1 claims that steps 2–4 have happened. A calibrated-reader success
is an exploratory measurement result, not retrospective NE-0c learning success.
