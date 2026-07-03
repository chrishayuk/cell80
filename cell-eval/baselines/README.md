# Recorded baselines

Reference runs the roadmap's numbers point at — **committed on purpose** (unlike
`results/`, the gitignored scratch directory for ad-hoc runs). Each file is the JSON
output of the corresponding `cell-eval` subcommand at a recorded point:

- `repair-granite4.1-3b.json`, `repair-gemma4-26b.json` — Phase-1.3 repair@1
  baselines (post-1.2 diagnostics): 0.60 / 0.90.
- `tier-calibration.json` — the ladder Item-1 margin-gate calibration on the seed
  library: the full θ curve per split and the chosen operating point (θ = 0.14,
  adversarial floor 0.75, potion embedder).
- `embed-bakeoff.json` — seven tier-2 embedders through the same gate + floor
  (potion / granite / nomic / embeddinggemma / qwen3-0.6b / mxbai / arctic2):
  nomic-embed-text is the recommended default (best answered-coverage per ms,
  the most-supported model on Ollama); qwen3-embedding:0.6b is the quality
  ceiling (ungated paraphrase 0.66, coverage 0.91/0.51/0.50); granite-embedding
  measured below both (paraphrase coverage 0.34) despite the stack preference.
  Retrieval prefixes were tested on the top three and don't change the ordering.

Re-record after a change that claims to move one of these (library growth, diagnostic
rewrites, index changes) and compare in the diff — drift is the signal.
