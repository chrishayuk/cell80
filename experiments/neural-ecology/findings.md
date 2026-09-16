# NE-0 findings: persistent local adaptation and temporal prediction

**Completed 2026-09-16. Ten evaluation seeds (2001–2010), with no exclusions, replacements, or parameter changes during evaluation.**

## Outcome

**NE-0 did not meet its registered learning criteria.** The Cell80 implementation passed execution and persistence checks and produced substantial persistent weight changes. The registered rule did not improve prediction over the identical frozen network, and the registered activity regime failed.

This is a result for the fixed neuron dynamics, topology, local rule, integer representation, linear measurement probe, grammar and adaptation budget. It does not establish that other local learning rules cannot work.

The [protocol](../neural-ecology-ne0-preregistration.md) was locally hash-sealed before implementation. All three development seeds were disclosed before evaluation; their weak results did not trigger tuning. The [seal history](seal-history.md) records a pre-evaluation file-cache correction that left the scientific protocol unchanged. This was not an externally timestamped registration.

## Held-out prediction

Each seed adapted for 50,000 ticks. B and C separately reconstructed the weights-only checkpoint, reset transient state, and used 256 warm-up ticks plus 10,000 examples. Separate probes were fitted on B for exactly 2,000 steps and frozen before C. The seed is the experimental unit: **n = 10**, not 100,000 test rows.

Lower log loss is better; units are nats per next-symbol prediction. Accuracy is descriptive.

| Arm/reference | Mean test loss | Mean accuracy |
|---|---:|---:|
| Frozen | 1.384368182 | 27.110% |
| Plastic | 1.385464169 | 25.975% |
| Yoked | 1.386852239 | 25.070% |
| Memoryless | 1.386974454 | 25.061% |
| Uniform current-symbol Bayes reference | 1.386294361 | 25% expected |
| Two-symbol oracle on C | 0.942413358 | 69.899% |

The population two-symbol Bayes loss is 0.940447989. Its empirical C loss fluctuates with sampled innovations. The fitted Memoryless arm is distinct from the exact uniform reference used for G6.

### All paired test losses

| Seed | Frozen | Plastic | Yoked | Memoryless | Oracle |
|---|---:|---:|---:|---:|---:|
| 2001 | 1.385826 | 1.387228 | 1.388583 | 1.388700 | 0.937529 |
| 2002 | 1.383627 | 1.384628 | 1.386003 | 1.386127 | 0.946480 |
| 2003 | 1.383688 | 1.384753 | 1.385990 | 1.386125 | 0.933443 |
| 2004 | 1.383846 | 1.384902 | 1.386048 | 1.386157 | 0.947259 |
| 2005 | 1.383391 | 1.384533 | 1.386063 | 1.386163 | 0.950567 |
| 2006 | 1.385646 | 1.386561 | 1.387657 | 1.387771 | 0.948232 |
| 2007 | 1.383864 | 1.384929 | 1.386340 | 1.386444 | 0.942005 |
| 2008 | 1.385004 | 1.386126 | 1.387766 | 1.387921 | 0.946869 |
| 2009 | 1.384409 | 1.385500 | 1.386743 | 1.386874 | 0.946286 |
| 2010 | 1.384379 | 1.385482 | 1.387329 | 1.387463 | 0.925464 |

### Registered contrasts

Positive differences favor Plastic. Each contrast requires both Bonferroni-adjusted one-sided sign-test p ≤ 0.05 and mean improvement ≥ 0.020 nats. The sign test measures consistency of improvement across seeds; it is not a test specifically of the mean.

| Control minus Plastic | Mean | Median | Sample SD | Positive seeds | Raw p | Adjusted p | Pass |
|---|---:|---:|---:|---:|---:|---:|---|
| Frozen | -0.001095987 | -0.001077745 | 0.000125387 | 0/10 | 1.000000000 | 1.000000000 | No |
| Yoked | 0.001388070 | 0.001365218 | 0.000231723 | 10/10 | 0.000976562 | 0.002929688 | No |
| Uniform reference | 0.000830193 | 0.001088857 | 0.000908489 | 8/10 | 0.054687500 | 0.164062500 | No |

All ten individual contrast values, accuracies, shuffled losses and oracle scores are retained in [evaluation-results.json](evaluation-results.json). No alternative test or favorable subset replaces these comparisons.

## G0–G6

| Gate | Result | Evidence |
|---|---|---|
| G0 | PASS | All ten runs complete; checked-i64 reference, Cell80-generated CPU, Metal and repeated Metal agree at every primary tick. No execution or probe failure. |
| G1 | FAIL | Plastic and Yoked exceed the 25% endpoint-weight limit; Yoked also exceeds the 0.35 spike-fraction limit. All seeds remain in the report. |
| G2 | PASS | Every Plastic seed exceeds both persistent-change minima: 1% changed weights and 0.10 mean absolute weight change. |
| G3 | FAIL | Plastic does not satisfy the improvement criteria against Frozen. |
| G4 | FAIL | The movement audit passes in every seed/sign, but the Plastic–Yoked effect does not reach 0.020 nats. |
| G5 | PASS | All 50 fresh-process replay checks and 80 frozen-probe identity checks pass, including learned-weight reconstruction and initial-weight restoration. |
| G6 | FAIL | Plastic does not satisfy both criteria against the exact current-symbol reference. |

G3 and G4 already use post-reset measurements. G5 validates persistence semantics; it is not an additional reset-versus-no-reset performance contrast. G1 is a failed scientific operating condition, while G0/G5 would indicate an invalid execution if they failed.

### Activity and persistent change

A activity is measured over its final 10,000 ticks. The JSON also includes scored B/C activity, every arm, requested/realised/clipped updates, per-sign histograms and clipping gaps.

| Seed | Plastic A spike fraction | Yoked A spike fraction | Plastic endpoint fraction | Yoked endpoint fraction | Plastic changed weights | Plastic mean absolute change |
|---|---:|---:|---:|---:|---:|---:|
| 2001 | 0.295750 | 0.453766 | 0.580322 | 0.580322 | 0.986938 | 15.801819 |
| 2002 | 0.288765 | 0.435302 | 0.572632 | 0.572632 | 0.988281 | 15.812134 |
| 2003 | 0.293101 | 0.440067 | 0.567810 | 0.567810 | 0.985229 | 15.801941 |
| 2004 | 0.293143 | 0.443031 | 0.598999 | 0.598999 | 0.987305 | 15.678040 |
| 2005 | 0.292657 | 0.441665 | 0.611938 | 0.611938 | 0.987244 | 15.818970 |
| 2006 | 0.289172 | 0.440303 | 0.581360 | 0.581360 | 0.984131 | 15.733215 |
| 2007 | 0.297615 | 0.449182 | 0.630310 | 0.630310 | 0.989380 | 15.845947 |
| 2008 | 0.289892 | 0.439223 | 0.602600 | 0.602600 | 0.988281 | 15.858276 |
| 2009 | 0.295912 | 0.449872 | 0.646179 | 0.646179 | 0.987793 | 15.787415 |
| 2010 | 0.289591 | 0.445900 | 0.625671 | 0.625671 | 0.987854 | 15.831421 |

### Conditional shuffled-state diagnostic

C neural-feature rows were shuffled within current-symbol groups using the registered fixed permutation, with the probe frozen. These differences are descriptive alignment checks, not independent significance tests.

| Arm | Mean normal loss | Mean shuffled loss | Shuffled minus normal |
|---|---:|---:|---:|
| Frozen | 1.384368182 | 1.387034012 | 0.002665831 |
| Plastic | 1.385464169 | 1.386992212 | 0.001528043 |
| Yoked | 1.386852239 | 1.386975784 | 0.000123545 |

## Execution, provenance and resources

The standalone [implementation](README.md) uses host-routed recurrent interactions with Cell80 node and plasticity cells. It makes no change to the acyclic `CellGraph`. The generated CPU backend is C++; CUDA hardware was not tested. Cross-backend byte identity applies to the neural computation. Float64 probe identity is checked in the recorded CPU environment, not promised across platforms.

Engineering verification passed 8 Rust tests, 8 Python tests and 700 primitive fixture cases. Primary evaluation compared 705,120 ticks per recurrent arm across four executions. Independent post-run auditing verified all 157 sealed file hashes, binary identity, phase counts, frozen B/C weights, probe/checkpoint evidence, all three statistical calculations, all seven gates and compressed artifact hashes.

Evaluation ran on Apple M3 Max / macOS 15.7.4 with three independent seed workers. Journal UTC timestamps span 393.70 minutes, while summed subprocess timers record 83.37 minutes across overlapping workers. The large timestamp gaps make that UTC span unsuitable as a compute-time measurement. All 121 journaled steps exited successfully; none was retried. Per-phase timing under shared-device contention is not an isolated backend benchmark.

Largest recorded primary-phase process RSS: 43.08 MiB. This includes that process's host allocations and mapped buffers, excludes probe-process peaks, and is not total device or workflow memory. Per-phase and per-probe times are in the JSON.

Protocol, implementation seal and compact findings are tracked. Full local streams, probes, checkpoints, tick hashes and interventions remain under `artifacts/eval-2001` through `artifacts/eval-2010`. Lossless archives retain both original and compressed hashes. Those bulky artifacts are ignored by Git and are not a separately published dataset.

| Artifact | SHA-256 |
|---|---|
| Protocol v1.0 | `f526a743eaefa4adf2a7f56721a5cfbcca3d4c502c64a54366b5c81c0eeb18d6` |
| Implementation seal v2 (committed before evaluation at `7e42374`) | `7d435fcd382fd06f519f7f64dde9cf443ccb57ab7c011af918da4e9a3c710a56` |
| Evaluation executable | `3745231d923db49990ca1971a01098290efa300104a4bdd55464ebb874b29ce3` |

## Interpretation

The execution substrate worked for this bounded recurrent experiment. The chosen adaptation rule moved weights substantially, but the resulting system missed its activity criteria and did not earn the registered persistent-learning claim. Saturation is an observed failure mode; these controls do not prove it caused the prediction failure.

Any next experiment should retain this result and use a separately frozen protocol and new evaluation seeds. Tuning the operating regime, changing the rule, or changing the representation/probe would answer a new question. NE-1 mechanism claims are not earned by this run.
