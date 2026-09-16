# NE-0b development findings: plasticity cadence

**Completed 2026-09-16. All twelve planned runs completed; no candidates or seeds were dropped.**

## Outcome

**Updating every 4,096 ticks was the only candidate that met every development criterion on all three seeds.** The original every-tick schedule reproduced the saturation and excessive Yoked firing seen in NE-0. Slower update schedules reduced those failures within the fixed 50,000-tick horizon.

This is an operating-regime result on development seeds. It does not demonstrate temporal learning or sustained stability under a longer adaptation budget. No prediction probes, test losses, or confirmatory streams were generated.

## Fixed design and selection

The [plan](plan.md) fixed seeds 3001–3003 and intervals 1, 16, 256 and 4,096 before the screen. Each paired run used the unchanged NE-0 neuron/plasticity cells, graph construction, initial weights, grammar, integer arithmetic and yoke permutation. Only the eligibility of a tick for a plasticity update changed.

A lasted 50,000 ticks. R reconstructed the weights-only checkpoint, reset all neuron state, froze weights and observed 10,000 ticks after 256 warm-up ticks. Candidates had to pass activity, endpoint, persistent-movement, movement-matching and execution checks in every seed. The smallest passing interval was selected. All intervals were completed before selection.

| Interval K | Update opportunities | Viable seeds | Plastic/Yoked endpoint fraction range | Maximum Yoked spike fraction across A/R |
|---|---:|---:|---:|---:|
| 1 | 50,000 | 0/3 | 0.5801–0.6269 | 0.4499 |
| 16 | 3,125 | 0/3 | 0.3812–0.3900 | 0.4447 |
| 256 | 195 | 0/3 | 0.2044–0.2640 | 0.3642 |
| 4,096 | 12 | 3/3 | 0.0320–0.0464 | 0.2294 |

The endpoint limit was 0.25; the spike-fraction limit was 0.35. At K=256, all three Yoked arms exceeded the spike limit after reset (0.3516–0.3642), even though two were below it during the last adaptation window. That candidate also exceeded the endpoint limit in two seeds.

## All twelve development results

| K | Seed | Activity | Endpoint limit | Persistent change | Movement match | Viable |
|---|---|---|---|---|---|---|
| 1 | 3001 | FAIL | FAIL | PASS | PASS | FAIL |
| 1 | 3002 | FAIL | FAIL | PASS | PASS | FAIL |
| 1 | 3003 | FAIL | FAIL | PASS | PASS | FAIL |
| 16 | 3001 | FAIL | FAIL | PASS | PASS | FAIL |
| 16 | 3002 | FAIL | FAIL | PASS | PASS | FAIL |
| 16 | 3003 | FAIL | FAIL | PASS | PASS | FAIL |
| 256 | 3001 | FAIL | FAIL | PASS | PASS | FAIL |
| 256 | 3002 | FAIL | FAIL | PASS | PASS | FAIL |
| 256 | 3003 | FAIL | PASS | PASS | PASS | FAIL |
| 4,096 | 3001 | PASS | PASS | PASS | PASS | PASS |
| 4,096 | 3002 | PASS | PASS | PASS | PASS | PASS |
| 4,096 | 3003 | PASS | PASS | PASS | PASS | PASS |

Execution passed for every row. Full per-phase, per-arm activity and per-sign requested/realised/clipped movement are retained in [development-results.json](development-results.json).

### Selected candidate: K=4096

| Seed | Plastic weights changed | Mean absolute weight change | Plastic endpoints | Yoked endpoints | Yoked R spike fraction |
|---|---:|---:|---:|---:|---:|
| 3001 | 0.839050 | 1.870605 | 0.046387 | 0.045654 | 0.229449 |
| 3002 | 0.811707 | 1.671570 | 0.040161 | 0.040100 | 0.227567 |
| 3003 | 0.779053 | 1.410339 | 0.034058 | 0.032043 | 0.226348 |

The selected schedule produced measurable persistent movement: 77.9–83.9% of Plastic weights changed, with mean absolute change 1.41–1.87 integer units. Thus its viability was not achieved by disabling updates entirely. All arms had at least 50% of units active and passed the rate limits in both measured windows.

However, K=4096 permits only twelve update opportunities. Reduced cumulative adaptation may explain much of the improvement in stability. The screen does not distinguish a useful slow-learning regime from a rule that will still saturate after more updates. It also does not establish that twelve opportunities can acquire useful predictive information. Periodic update timing can interact with recurrent dynamics; it is not equivalent to scaling every update continuously.

## Execution and audit

- Three Rust tests and six Python tests passed; all 700 inherited primitive fixtures matched NE-0.
- Every tick compared independent checked-i64 reference, Cell80-generated C++ CPU, Metal and repeated Metal worlds, including currents, requested deltas, neuron state and weights.
- Across twelve runs, this covered 723,072 ticks per recurrent arm. All twelve process exits were successful.
- Frozen histories, topology and streams matched exactly across all four intervals within each seed.
- Persistent-only reconstruction, idempotent reset and zero R weight movement passed.
- The independent post-run audit recomputed acceptance from numerical statistics, verified artifact hashes and confirmed the original NE-0 seal remained unchanged.

These are full-state comparisons inside each candidate process. NE-0b does not claim a new fresh-process probe validation: there is no probe in this study. The generated C++ backend is not CUDA hardware.

The first fixture attempt inside the restricted sandbox could not see a Metal device. After approved GPU access, fixtures passed. No neural candidate run failed or was retried.

## Provenance and next boundary

The source and plan were committed at `87ec6e0`; the implementation seal was committed at `35d5713` before the first candidate run. The [implementation instructions](README.md) describe reproduction and artifact layout.

| Artifact | SHA-256 |
|---|---|
| Development plan | `9634eb48f421dd6bc4235094f3ce36565dd35dbc6898278a928565657e919dec` |
| Implementation seal | `1a65b6f599f69c21de63d5db8fd7b4cf04b65dba159caf1af5200c2b05ab4d5d` |
| Executable | `82d2e03ce355e6165ed1296ea219390bf1bd5140f652a6a115fa53afacde83eb` |

Raw snapshots, symbol streams, tick hashes and attempt logs remain locally under `artifacts/`; their hashes are in the tracked numerical report. Per-phase times include host routing, equality checks, hashing, checkpoint I/O and shared-device contention. They are not isolated backend benchmarks.

**Next scientific question:** with K=4096 fixed, does activity-dependent adaptation improve held-out temporal prediction over Frozen and Yoked after transient reset? That requires a separately frozen learning protocol and fresh confirmatory seeds. The selected operating regime has not yet earned an acquisition claim. NE-0’s original negative result remains unchanged.
