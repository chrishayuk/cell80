# Recorded baselines

Reference runs the roadmap's numbers point at — **committed on purpose** (unlike
`results/`, the gitignored scratch directory for ad-hoc runs). Each file is the JSON
output of the corresponding `cell-eval` subcommand at a recorded point:

- `repair-granite4.1-3b.json`, `repair-gemma4-26b.json` — Phase-1.3 repair@1
  baselines (post-1.2 diagnostics): 0.60 / 0.90.
- `tier-calibration.json` — the ladder Item-1 margin-gate calibration on the seed
  library: the full θ curve per split and the chosen operating point (θ = 0.14,
  adversarial floor 0.75).

Re-record after a change that claims to move one of these (library growth, diagnostic
rewrites, index changes) and compare in the diff — drift is the signal.
