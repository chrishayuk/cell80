# NE-1b intervention specification

**Version 1.0 — 2026-09-18. Exploratory, frozen before intervention measurements.**

## 1. Question and limits

Do endpoint weight changes causally contribute to the observed accessibility of
temporal targets? Do reversions affect an original reader differently from a
refitted reader, and do those effects differ between Plastic and Yoked?

These targets were selected after the complete, audited NE-1a results. Seeds,
streams and hypotheses are reused. This is an exploratory counterfactual study,
not an independent replication or a retrospective change to NE-0c. No new
adaptation, local rule, projection analysis, edge-effect search, internal readout
population, or changed neuron dynamics is included.

## 2. Inputs and immutable provenance

Evaluation seeds **4001–4010**; separate development seed **3001**. Endpoint
adaptation tick **50000**, cadence **4096**, N=1024 and E=16384. Use the unchanged
NE-1a archived topology and initial/endpoint persistent weights. Initial means
requested tick zero (archived tick 1000, before any eligible weight update).
Require all initial arms to agree and endpoint Frozen to equal initial weights.
Check source archive hashes, NE-1a fit-manifest membership for every imported
checkpoint and reader file, inherited code seal, and completed passing NE-1a
matrix/audits. Record all imported file hashes in the inventory and execution seal.

An edge ID is its position in the archived `sources` array: destination is
`edge_id // 16`; source is `sources[edge_id]`. Source sign is +1 when source <768,
otherwise −1. Stored weights are magnitudes in [0,32]; source signs never change.
Define signed magnitude change `d[e] = endpoint[e] - initial[e]`, and changed
set `D = {e: d[e] != 0}` separately for each seed and endpoint arm.

Only selected weights change: `w_reverted[e]=initial[e]` for selected e, otherwise
`endpoint[e]`. No interpolation, scaling, deletion, clipping, or reapplication
of requested STDP deltas. These are endpoint reversions, not removal of a
historical update event. Topology, source signs and every other parameter remain
fixed. Reset **all v/r/s/q fields to zero before each stream**, and reconstruct
all other transient fields exactly as NE-1a did. Plasticity stays off.

## 3. Selection, branches and random controls

Percentages use **all 16384 edges as denominator**, while selections contain only
changed edges. Counts are `ceil(E*p/100)`: **164, 820, 1639** for p=1,5,10.
Fewer changed edges than required is a design/integrity failure; never resize.

For each seed and arm sort changed edges by decreasing `abs(d[e])`. Break ties
by ascending raw SHA-256 digest of the ASCII string
`NE1b-v1/{seed}/top/{edge_id}`, then ascending integer edge ID if digests tie.
Integers have ordinary decimal formatting without padding or trailing newline.
Take the first k edges. Thus the three top sets are nested. The random tie-break
does not select an edge because of neuronal activity or predictive outcome.

There are two random families, each with **replicates 0,1,2,3,4** at every size:

| Family | Stratum | Exact match to corresponding top set | Does not match |
|---|---|---|---|
| `count` | (source sign, sign(d)) | number of edges in each stratum | magnitude of movement |
| `movement` | (source sign, d) | counts and complete signed-change histogram per source sign; hence L1 and squared-L2 movement | endpoints, source/destination identities, topology motifs, initial/endpoint weight levels |

Within each stratum, sample without replacement from **all changed edges in that
stratum**, including top-set edges. Rank candidates by ascending raw SHA-256 of
`NE1b-v1/{seed}/{family}/{replicate}/{edge_id}`, then integer edge ID. Select the
number present in that stratum of the top set. The address intentionally omits
arm and fraction: P/Y share random ranks on common edges, and selections are
nested across fractions within a family/replicate/arm. Stratum memberships can
differ between arms; paired replicates do not imply identical selected edges.

No rejection sampling, redraw, disjointness constraint or best-random selection.
Top sets themselves prove matching feasibility. Repeated or identical random
subsets remain valid logical controls and are retained. These are five technical
draws, not five additional experimental units.

Exact movement matching of a top-magnitude set necessarily includes all edges
above its cutoff and randomizes mainly cutoff ties. It cannot furnish an
independent test of the importance of every large-change edge. For every subset
report top overlap, Jaccard similarity, actual L1 and squared-L2 movement, signed
sum, source-sign/change-direction counts, and the number of possible matched
sets (sum of log2 binomial counts over strata). Report the number of forced edges
and distinct realized draws. Never infer address specificity from identical
selections. Count-control differences can reflect movement dose as well as edge
identity. Do not divide loss differences by movement and interpret the ratio as
a linear causal effect.

Per endpoint arm: **35 branches** = unchanged (empty subset), all (D), top1/5/10,
and 3 sizes ×2 families ×5 replicates. Shared Frozen has initial weights. Do not
run extra fractions, rank by downstream losses, or remove unsuccessful controls.
Selection requires only archived weights/topology and fixed random addresses;
no fit, validation, test labels, activations or coefficients enter selection.

## 4. Streams and two measurement instruments

Reuse **exact NE-1a fit/validation/test streams**, including original namespace,
grammar, 256 unscored warm-up ticks, 10000 scored rows, and post-current-input
indexing. Each stream has 10257 symbols; scored t=256…10255. Every branch in a
seed sees the identical stream and starts from zero transient state. Existing
test labels and NE-1a outcomes have already been observed. Freeze all new branch
fits across all evaluation seeds before generating **any NE-1b intervention test
trajectory** or reading its scores. Reusing old symbols is not a new-data claim.

Only two readouts, both q=[q0,…,q1023]:

* **A:** G, target x[t−2], no current-symbol side input. One four-class softmax head.
* **B:** C, target x[t+1], four independent four-class heads routed by x[t].
  Routing has the same explicit current-symbol access as NE-1a.

Apply [NE-1a measurement-spec sections 2–6](../neural-ecology-ne1/measurement-spec.md)
unchanged for these normalized readers: fit-only mean/std (ddof=0), constant
threshold 1e−12 with zero output on every split, whole-fit normalization shared
by C heads, float64, unregularized intercepts, coefficient L2, grid
**[0.0001,0.001,0.01,0.1,1]**, zero initialization, L-BFGS-B with **maxiter=2000,
maxfun=20000,maxcor=20,maxls=50,gtol=1e−6,ftol=0**, pooled validation loss for C,
tie tolerance **1e−8** selecting largest lambda, and no fit+validation refit.
Keep the pinned numerical environment and single BLAS/OpenMP thread. All five
candidates must pass the original convergence/finite/objective criteria.

Modes:

1. **Refitted:** fit independently on each branch's fit features and select using
   that branch's validation data. Fit-only normalization belongs to that branch.
2. **Transported:** import that seed's NE-1a **Frozen time-zero** reader, including
   coefficients, intercepts, lambda, constant mask and normalizer unchanged.
   No endpoint-trained fixed reader, recentering, coordinate alignment or
   recalibration is introduced.

The reader is an external supervised instrument. Changes in refitted performance
measure accessibility under that instrument, not total information or endogenous
learning. Restoring the old reader's performance alone is not proof of pure
rotation, and worse refitted loss is not proof of information destruction.

## 5. Identity, calibration and execution gates

Before neural measurements commit this specification, inventory and specification
seal. The seal describes only this stage. Next implement a separate runner;
preserve every sealed NE-0/NE-0b/NE-0c/NE-1a file. Seal its source/binary/environment
before development measurement. A code change invalidates that implementation
seal and requires a recorded new version, never a silent replacement.

Development uses the **complete 71-system, two-reader, two-mode matrix** on seed
3001. Before development trajectories, rerun the inherited q-width G/lag2 copying
and C/next known-previous-symbol calibration controls using the unchanged streams
and numerical rules. Required gates: copying accuracy >=.999 and loss <=.010;
conditioned next excess loss above sampled oracle <=.010 and accuracy >=.65.
Validate feature/target indexing and every intervention selector/reversion.

For development and evaluation require:

* Empty reversion equals endpoint persistent bytes; all reversion equals Frozen.
  Only selected IDs may differ from endpoint; no transient state is imported.
* Reference, generated CPU, Metal and repeated Metal agree on every tick, with
  zero plasticity requests and unchanged persistent weights during measurement.
* Newly executed unchanged/Frozen/all-reverted trajectories match their NE-1a
  reference feature and replay hashes for each identical stream. Do not satisfy
  this gate only by pointing to an old file. Execute each identity branch once.
* Once trajectory identity is established, refits must reproduce inherited
  normalization, candidate validity/losses, selected lambda, coefficients and
  prediction bytes in the same numerical environment. Wall-clock timing metadata
  is excluded. Transported prediction reproduction uses saved original readers.
* Reproduce the two linked q-width calibration results on every evaluation seed
  from unchanged saved models/arrays; all **20 evaluation controls** must pass.
  Their provenance includes the completed NE-1a calibration/audit. No extra neural
  readout families are introduced by this calibration check.

After complete passing development, seal implementation and validation evidence
before new evaluation trajectories. Complete all fit/validation trajectories and
all candidate fits, retaining failures, across all ten seeds. Commit/hash the
global fit manifest before intervention test trajectories. Then complete the
whole test matrix and independently reproduce scores/contrasts from saved arrays,
coefficients and selected edge lists before interpretation.

Any candidate failure invalidates its entire refitted cell; no surviving-lambda
selection, retry or fallback. Retain null rows and continue independent cells.
Invalid inherited transported models invalidate their dependent rows. Parity,
reset, selection, calibration or identity failures invalidate the corresponding
trajectories/interpretations and dependent contrasts; do not reinterpret a failed
identity check as a scientific effect. Preserve failures, finish unaffected planned
cells, and mark the matrix incomplete/invalid as appropriate. A development
integrity/calibration failure blocks evaluation. External interruptions permit
only logged exact-configuration resumes/reruns, preserving partial attempts.
No seed replacement, optional stopping, tolerance relaxation or outcome-driven
amendment within v1. Cache exact duplicates only after identity verification;
keep all logical rows, subset hashes and provenance.

## 6. Endpoints, contrasts and output matrix

Primary measurement is test mean log loss (nats). Accuracy is descriptive. This
exploratory study has **no new significance or learning-success threshold**.
Seeds, not timesteps or random subsets, are the units. Report every per-seed
contrast and complete-cohort mean, median, sample SD and positive count. An
incomplete contrast has null cohort summaries plus its valid count, not a
favorable-subset average.

For each target/mode and seed, let L(a,b) be arm a's loss under branch b, U its
unchanged branch, T_p the top set, and R_f,p,r a random subset.

* Reversion effect **D(a,b)=L(a,b)−L(a,U)**: positive means those retained endpoint
  changes helped that reader (reverting them hurt); negative means reversion helps.
* Baseline gain **L(Frozen)−L(a,U)**. All-reversion effect must equal this value.
  This is an identity/reproduction control, not a new independent discovery.
* Top versus random **D(a,T_p)−mean_r D(a,R_f,p,r)**, separately by random family.
  Average all five draws within seed first; any missing draw makes it incomplete.
* Plastic–Yoked specificity **D(Plastic,b)−D(Yoked,b)**, and the analogous difference
  in the top-versus-random contrasts. Positive means the reversion cost was larger
  for Plastic. Endpoint-based selection differs by arm; this is the effect of
  matched *selection policies*, not the same edge intervention in both arms.
* Reader dependence **D(a,b,transported)−D(a,b,refitted)**, reported descriptively
  within a target. No averaging the two targets or two reader modes together.

All these are finite, joint-subset interventions; do not sum them into individual
edge effects or claim mediation fractions. A Plastic–Yoked difference supports
a distinction between these realized endpoint systems, not a universal advantage
of activity dependence or isolation from every clipping/dynamics difference.

| Logical output | Evaluation | Separate development |
|---|---:|---:|
| Systems (1 Frozen + 2×35 per seed) | 710 | 71 |
| Test score rows (2 readouts ×2 modes) | **2840** | 284 |
| Refitted readout cells | 1420 | 142 |
| Candidate rows (5 per refitted cell; heads nested) | **7100** | 710 |
| Linked calibration score rows | 20 | 2 |
| Reversion-effect rows (34 nonempty branches ×2 arms ×2 targets ×2 modes) | 2720 | 272 |
| Baseline-gain rows | 80 | 8 |
| Top-versus-random-mean rows (2 families ×3 sizes ×2 arms ×2 targets ×2 modes) | 480 | 48 |
| Plastic–Yoked reversion-effect differences | 1360 | 136 |
| Plastic–Yoked top-versus-random differences | 240 | 24 |
| Reader-dependence rows (34 branches ×2 arms ×2 targets) | 1360 | 136 |

Retain all selected IDs (sorted ascending), subset hash (little-endian u32),
resulting weight hash (little-endian u16), match/overlap/movement diagnostics,
source and arm, every split's feature/stream/replay hashes, candidate diagnostics,
normalizer/model hashes, selected lambda, probability hashes and calibration
status. Save q features sufficient to independently reproduce every fit and score;
retain full-state replay hashes and activity diagnostics. Record parameter counts
(G:4100, C:16400), current-symbol access, firing rates and q moments/constants.
The uniform reference is ln(4); also report the same sampled next-symbol oracle
and inherited fit-only current-symbol baseline for both targets. No new fitted
baselines or reader families.

## 7. Interpretation and prospective follow-up

Improved transported loss with stable refitted loss supports a change in the
relationship between representation and original reader. Worsened refitted loss
after reversion supports a contribution of those endpoint changes to accessibility
under this instrument and stream. Random controls qualify whether this depends
on movement dose, tied-edge choices, or broader changed-edge selection; they do
not automatically isolate a special synaptic mechanism.

Reversions can introduce activity changes; report these rather than calling every
decoding change a stored-memory effect. Preserve null, mixed and opposing reader
results. The two selected phenomena are not the whole NE-1a matrix. NE-0c remains
negative under its registered instrument, and NE-1a remains exploratory. Any
mechanism or remedy selected here needs a separately frozen fresh-seed study.
