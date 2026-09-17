# NE-0c findings: confirmatory learning at K=4096

**Completed 2026-09-17. All ten fresh evaluation seeds (4001–4010) completed. No seed exclusions, replacements or scientific tuning.**

## Primary outcome

**NE-0c did not meet the registered learning criteria.**

The selected cadence preserved viable activity and changed 77.6–82.5% of Plastic's
weights. Plastic nevertheless performed worse than Frozen in eight of ten seeds.
Its small advantage over Yoked did not pass the registered statistical test.

The [preregistration](preregistration.md) fixed the NE-0b-selected cadence while retaining NE-0’s rules, topology, integer state, arm semantics, clipping, grammar, measurement probe and decision thresholds. This was a locally hash-sealed protocol, not an externally timestamped registration. The original NE-0 and NE-0b results remain intact.

Each network received 50,000 adaptation ticks and exactly twelve update opportunities. All weights were then frozen. Primary B and C independently reconstructed weights-only checkpoints, reset all transient state, and used 256 warm-up ticks followed by 10,000 examples. Separate fixed probes were fitted on B, frozen before C, and never updated from C. The experimental unit is the seed: n=10.

## Held-out prediction

Lower loss is better; units are nats per next-symbol prediction. Accuracy is descriptive.

| Arm/reference | Mean primary C loss | Mean accuracy |
|---|---:|---:|
| Frozen | 1.385166335 | 26.121% |
| Plastic | 1.385301100 | 25.959% |
| Yoked | 1.385405921 | 25.988% |
| Memoryless | 1.387703289 | 24.515% |
| Uniform current-symbol Bayes reference | 1.386294361 | 25% expected |
| Two-symbol oracle on actual C | 0.938755047 | 70.087% |

The population full-history Bayes loss is 0.940447989; its empirical loss depends on sampled innovations. The exact uniform reference, rather than the fitted Memoryless arm, defines G6.

### All ten paired primary results

| Seed | Frozen loss | Plastic loss | Yoked loss | Memoryless loss | Frozen − Plastic | Yoked − Plastic |
|---|---:|---:|---:|---:|---:|---:|
| 4001 | 1.384728980 | 1.384961811 | 1.384973716 | 1.387002982 | -0.000232830 | 0.000011905 |
| 4002 | 1.388799898 | 1.389037216 | 1.389142210 | 1.391217036 | -0.000237319 | 0.000104993 |
| 4003 | 1.384582005 | 1.384452346 | 1.384697110 | 1.386906584 | 0.000129659 | 0.000244764 |
| 4004 | 1.384575962 | 1.384829796 | 1.384765478 | 1.386859127 | -0.000253834 | -0.000064319 |
| 4005 | 1.383980242 | 1.383931686 | 1.384111697 | 1.386530711 | 0.000048555 | 0.000180011 |
| 4006 | 1.385223967 | 1.385504738 | 1.385497649 | 1.388015096 | -0.000280771 | -0.000007089 |
| 4007 | 1.385260058 | 1.385351383 | 1.385370950 | 1.387946889 | -0.000091325 | 0.000019567 |
| 4008 | 1.384629812 | 1.384726800 | 1.384964473 | 1.386959852 | -0.000096988 | 0.000237673 |
| 4009 | 1.384707151 | 1.384889775 | 1.385068111 | 1.387661611 | -0.000182623 | 0.000178337 |
| 4010 | 1.385175279 | 1.385325445 | 1.385467815 | 1.387933008 | -0.000150166 | 0.000142370 |

### Registered tests

Positive differences favor Plastic. Plastic versus Frozen and versus Yoked are the primary causal contrasts; the G6 reference contrast stays in NE-0’s unchanged family of three. Each requires Bonferroni-adjusted one-sided sign-test p≤0.05 and mean improvement≥0.020 nats. The sign test tests consistency across seeds, not specifically the mean. The minimum mean effect is a separate condition.

| Control minus Plastic | Mean | Median | Sample SD | Positive seeds | Raw p | Adjusted p | Pass |
|---|---:|---:|---:|---:|---:|---:|---|
| Frozen | -0.000134764 | -0.000166395 | 0.000135400 | 2/10 | 0.9892578125 | 1.0000000000 | FAIL |
| Yoked | 0.000104821 | 0.000123682 | 0.000108901 | 8/10 | 0.0546875000 | 0.1640625000 | FAIL |
| Uniform reference | 0.000993261 | 0.001368568 | 0.001391845 | 9/10 | 0.0107421875 | 0.0322265625 | FAIL |

All individual d_M values, accuracies, conditional shuffled losses, oracle scores and probe objectives are retained in [evaluation-results.json](evaluation-results.json). No alternative inferential test or favorable subset replaces the registered comparisons.

The uniform-reference contrast passes the consistency test (adjusted p=0.03223),
but its mean advantage of 0.000993 nats is about twenty times smaller than the
required 0.020. G6 therefore fails on effect magnitude. This is evidence of a
small readable predictive signal, without evidence that plasticity improved that
signal relative to the frozen recurrent system.

## G0–G6

| Gate | Result | Evidence |
|---|---|---|
| G0 | PASS | All ten complete; reference/generated CPU/Metal/repeated Metal agree at each primary tick; valid finite probes. |
| G1 | PASS | Primary measured spike fractions 0.2158–0.2361; maximum final-A endpoint fraction 0.0446. All per-arm activity records are in the JSON. |
| G2 | PASS | Plastic changed-weight fractions 0.7758–0.8248; mean absolute changes 1.4066–1.7603. |
| G3 | FAIL | Requires the d_F statistical and minimum-effect criteria shown above. |
| G4 | FAIL | Movement audit passes; the d_Y contrast must separately pass both statistical and effect criteria. |
| G5 | PASS | All 50 fresh-process replays and 80 frozen-probe identity checks pass; primary initial neuron-state hashes are the canonical zero state. |
| G6 | FAIL | Requires the d_M statistical and minimum-effect criteria shown above. |

G3/G4 already measure post-reset performance. G5 verifies persistence semantics; it is not an independent statistical reset-versus-no-reset test. Gate failure does not by itself mean zero effect. In particular, an effect can pass the consistency test while missing the minimum magnitude.

## Secondary continuation diagnostic

For B and C independently, the diagnostic restored each full final-A snapshot and omitted transient reset. It used the same corresponding B/C symbols, warm-up and fixed probe-fitting procedure, with separately fitted diagnostic probes. Diagnostic C started from final A, not from diagnostic B. Weights remained frozen throughout.

| Arm | Primary mean loss | Continuation mean loss | Continuation minus primary |
|---|---:|---:|---:|
| Frozen | 1.385166335 | 1.385168680 | 0.000002345 |
| Plastic | 1.385301100 | 1.385298532 | -0.000002568 |
| Yoked | 1.385405921 | 1.385411486 | 0.000005565 |
| Memoryless | 1.387703289 | 1.387703289 | 0.000000000 |

| Seed | Continuation Frozen − Plastic | Continuation Yoked − Plastic |
|---|---:|---:|
| 4001 | -0.000235824 | 0.000042450 |
| 4002 | -0.000217295 | 0.000096904 |
| 4003 | 0.000136777 | 0.000239293 |
| 4004 | -0.000237191 | -0.000065584 |
| 4005 | 0.000016000 | 0.000194574 |
| 4006 | -0.000288191 | 0.000005102 |
| 4007 | -0.000085608 | 0.000031302 |
| 4008 | -0.000103551 | 0.000234614 |
| 4009 | -0.000137064 | 0.000231869 |
| 4010 | -0.000146571 | 0.000119016 |

Mean continuation contrasts: Frozen − Plastic = -0.000129852; Yoked − Plastic = 0.000112954 nats.

These secondary differences are descriptive, with no confirmatory tests or additional success gate. They cannot rescue the primary verdict. All continuation starting-state hashes matched their full final-A snapshots, while primary B/C started at zero. The 256-tick warm-up may erase transient effects; little difference between branches does not prove transient state is never useful.

## Engineering and provenance

Before evaluation, all 700 inherited primitive GPU fixtures, two new Rust tests and five aggregation tests passed. The full pipeline ran on development seed 3001 without tuning. Its full adaptation checkpoint exactly matched NE-0b K=4096, including topology, weights and transient neuron state. Those validation scores are [disclosed separately](development-results.json) and do not enter confirmatory inference.

The protocol was committed at `bb90424`, implementation at `262b90f`, and validated seal at `effb845`, before any evaluation stream/network was generated. The [pre-launch history](launch-history.md) records sandbox-related fixture/metadata failures and an evaluation entry refusal before the seal existed; no evaluation seed ran in those failed preflights.

Independent post-run auditing verified 196 sealed files, the binary, all ten seed identities, frozen weights, reset/continuation initial states, replay evidence, probe hashes, numerical movement statistics, the three exact test calculations, all gates and compressed artifact hashes. All 160 required evaluation subprocess steps ultimately succeeded. A disk-space interruption required three explicitly logged continuation reruns; their retained completed tick prefixes exactly match the reruns. Three queued A starts had no recorded exits or artifacts and were rerun. See [recovery history](recovery-history.md) for the preserved evidence and orchestration changes.

The retained [audit implementation](audit.py) also independently recomputed all
800,000 primary/secondary predictions from saved C arrays and frozen coefficients,
matching probability hashes, losses and accuracies, and recomputed the oracle
scores. It does not fit probes or run new neural trajectories.

Primary execution compared 705,120 ticks per recurrent arm across four executions; continuation added 205,120 ticks per arm. The generated CPU backend is C++, not CUDA hardware. Neural trajectory identity is exact; float64 probe identity is checked in the recorded numerical environment, not asserted across platforms.

Hardware: Apple M3 Max; macOS 15.7.4; NumPy 2.4.2. Initial execution used three independent seed workers; recovery used sequential execution. Per-phase/probe times and process RSS are retained in the JSON. They include routing, hashing, I/O and shared-resource contention and are not isolated compute benchmarks. RSS does not capture all device memory or the full workflow peak.

| Artifact | SHA-256 |
|---|---|
| Protocol | `26247783edf6d65c993ccac1b6e17f3335fb7b3193ea8b66b1a7fa43b55b2775` |
| Implementation seal | `44289523cb533c7884f775161b5a152ae909235d8a84c60ace4fbe23b760bae1` |
| Evaluation binary | `19352b79e257682c2fffc2d11684562ebd5826c47d15f54e5ac705082e52d711` |

Full local artifacts remain under `artifacts/eval-4001` through `artifacts/eval-4010`, including streams, primary arrays/probes, checkpoints, tick hashes and losslessly archived replay/continuation products. Original and compressed hashes are retained. These bulky artifacts are ignored by Git and have not been independently published.

## Bounded conclusion

Under the frozen neuron dynamics, topology, cadence, plasticity rule, integer representation, probe and adaptation budget, NE-0c did not meet the specified criteria for persistent, activity-specific temporal learning. The failed gates and effect sizes above define that result; it does not establish that local learning in general cannot work.

Twelve update opportunities produced measurable persistent changes, but the number of changed weights alone does not establish useful temporal information. Any further model, rule, representation, probe or budget change requires a new experiment and fresh evaluation seeds.
