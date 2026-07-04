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
  - Checkpoint 3 (`checkpoint-3-gsm8k-checked-arithmetic`, 128 cells, GSM8K math campaign
    M1 pack 1/5): P@1 direct 0.95 / paraphrase 0.43 / adversarial 0.41 — flat within noise
    (paraphrase 0.4304→0.4253 on a denominator that grew 79→87 queries). Kill-gate did not
    trigger.
  - Checkpoint 4 (`checkpoint-4-gsm8k-money-bps`, 134 cells, GSM8K math campaign M1 pack
    2/5): P@1 direct 0.94 / paraphrase 0.46 / adversarial 0.44 — both paraphrase and
    adversarial ticked up from checkpoint 1's baseline (0.42/0.39), direct flat. Kill-gate
    did not trigger.
  - Checkpoint 5 (`checkpoint-5-gsm8k-units`, 138 cells, GSM8K math campaign M1 pack 3/5):
    P@1 direct 0.95 / paraphrase 0.45 / adversarial 0.38 — adversarial dipped below
    checkpoint 1's baseline (0.39). Traced to the exact 2 flipped cases (of 34): a
    corpus-wide TF-IDF weight shift re-ranked two *pre-existing* confusable pairs
    (`percent_to_byte`/`byte_to_percent`, `accumulate_step`/`mean3`) — neither involves a
    units-pack cell, and the units pack's own 8 direct/paraphrase cases hit 7/8 (the one
    miss, `unit_mul`/`unit_div` under paraphrase, is an ordinary same-shape-sibling miss).
    Not attributable to a units-pack collision; kill-gate did not trigger, but this is the
    first checkpoint to dip under the baseline on any split and is worth watching if the
    trend continues.
  - Checkpoint 6 (`checkpoint-6-gsm8k-verifier-ranker`, 142 cells, GSM8K math campaign M1
    pack 4/5): P@1 direct 0.95 / paraphrase 0.45 / adversarial 0.41 — adversarial recovered
    above checkpoint 1's baseline (0.39), confirming checkpoint 5's dip was the flagged
    IDF-reordering noise, not a trend. Kill-gate did not trigger.

Re-record after a change that claims to move one of these (library growth, diagnostic
rewrites, index changes) and compare in the diff — drift is the signal.
