# NE-1b development validation

**Passed 2026-09-18, seed 3001 only. No evaluation outcomes used.**

Scientific specification commit: `b6ec99f`. Validated implementation commit:
`1d5ec61`. The specification hash remains
`82af5adfe1dbb695886126922f8dbf97cf7a4adccff60aae3dea90668ebd3aeb`.

| Gate | Evidence |
|---|---|
| Selector and orchestration tests | Eight pass |
| Inherited GPU fixtures | All 700 match; repeated Metal agrees |
| Generalized batch step | Exactly matches unchanged NE-0c step for 32 fixture ticks |
| Synthetic calibration | Both required controls pass |
| Full development matrix | 71 systems ×2 readouts ×2 modes =284 valid rows |
| Candidate fitting | 710 logical records; 690 distinct candidates, all valid |
| Neural execution | Reference/generated CPU/Metal/repeated Metal agree at every tick |
| State and parameter integrity | Reset, zero-change, all-reversion, and unchanged-weight checks pass |
| Archive identity | Unchanged and fully reverted trajectories reproduce NE-1a |
| Reader identity | All ten identity readouts reproduce original normalization, candidate coefficients/losses and selection |
| Fit/test barrier | Manifest frozen before test trajectory generation; all bound files still match |
| Implementation integrity | All 1,390 sealed files unchanged |
| Independent audit | 284 rows, 276 distinct model/feature predictions, 138 models, 690 candidate checks, 624 contrasts, two calibrations, 20 identity predictions reproduced |

Fit manifest SHA-256:
`d24a252d13a45546fe8aba4062067264f06c70092a63d775bb6f1fd0124e1665`.
Development code-seal SHA-256:
`a95cd053163b971f1fdd1116b69dd44a0a17d3a3c0824cda029675155833bd19`.

The first implementation attempt was stopped before test because its pre-test
manifest would have included a subsequently appended command log. The original
code, seal and partial artifacts are retained. Moving logs outside the sealed
seed directory was the only runner correction. Restarted trajectories match all
complete original records: 3,524 fit ticks and 3,311 validation ticks. No original
attempt output was substituted for completed validation.

These are implementation gates, not a scientific NE-1b result. Development seed
3001 will not be pooled with evaluation. The [results](development-results.json),
[independent audit](development-independent-audit.json),
[complete scores](development-scores.csv), and [tables](development-tables.json)
are retained. Evaluation requires its separate committed seal before execution.
