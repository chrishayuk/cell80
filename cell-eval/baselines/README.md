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

- `tier3-granite4.1-3b.json`, `tier3-gemma4-26b.json` — the tier-3 probe-evidence
  A/B over the escalated residue (nomic tier 2, θ = 0.05): a **banked negative** for
  raw probe tables on text-only escalations. The 26B resolves the pickable residue
  at 1.00 from manifests alone (probes neutral); the 3B sits at 0.85–1.00
  manifests-only and probes *hurt* (−0.11…−0.38). Behavioural probes stay for
  example-carrying requests (`match_examples`) and register-time metadata — not as
  escalation payload.

- `library-scale-curve.json` — Phase 2.3's retrieval-quality-vs-scale curve: one record
  per checkpoint, appended by `cell-eval curve` (`cell-eval/src/cell_eval/curve.py`), each
  a real run against `cell80/cells/` as it stood at that commit — never a fabricated
  point. Adoption/composition are `{"skipped": "..."}` when no model endpoint is
  configured, not faked.
  - Checkpoint 1 (`checkpoint-1-wave3-complete`, 114 cells): P@1 direct 0.94 / paraphrase
    0.42 / adversarial 0.39.
  - Checkpoint 2 (`checkpoint-2-pilot-batch`, 120 cells, the first author→verify→admit
    pilot batch): P@1 direct 0.95 / paraphrase 0.43 / adversarial 0.41 — **no split
    degraded**, all three ticked up slightly. The kill-gate
    (`docs/library-growth.md` "Phase 2.3") did not trigger.

Re-record after a change that claims to move one of these (library growth, diagnostic
rewrites, index changes) and compare in the diff — drift is the signal.
