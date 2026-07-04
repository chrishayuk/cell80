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
  - Checkpoint 7 (`checkpoint-7-stateful-rng`, 145 cells, first stateful/RNG slice —
    library-growth.md "Next waves", outside the GSM8K campaign): P@1 direct 0.95 /
    paraphrase 0.44 / adversarial 0.41 — all three above checkpoint 1's baseline. Kill-gate
    did not trigger.
  - Checkpoint 8 (`checkpoint-8-signed-deltas`, 149 cells, first signed-deltas slice):
    P@1 direct 0.94 / paraphrase 0.44 / adversarial 0.41 — direct dipped a hair under
    checkpoint 1's baseline (0.9363 vs 0.9426) for the first time, one flipped case (of
    157): `abs_i16`'s own summary shares "absolute value" with `abs_diff`'s direct query
    ("compute the absolute value of a minus b"), so it now edges out `abs_diff` for that
    one query — a real, explainable same-family collision, not noise, but exactly the
    expected cost of growing a confusable family the project's own pitch accepts (both
    cells still surface in the top-3). Paraphrase/adversarial both stayed above baseline.
    Kill-gate did not trigger.
  - Checkpoint 9 (`checkpoint-9-scoring-choice`, 153 cells, scoring/choice second slice):
    P@1 direct 0.93 / paraphrase 0.42 / adversarial 0.41. Direct's own second consecutive
    dip below checkpoint 1's baseline (0.9255 vs 0.9426) — this time attributable to the
    new pack itself: `weighted_sum2`/`weighted_sum3`'s own direct queries both rank the
    shorter, pre-existing `weighted_sum` #1 instead of themselves (both still land in
    hit@3, which held steady). Paraphrase dipped fractionally under baseline for the
    first time too (0.4196 vs 0.4247) — all three flipped paraphrase cases are
    pre-existing pairs re-ranked by the usual corpus-wide TF-IDF shift
    (`range_check`/`between_exclusive`, `weighted_sum`/`choose_best3`,
    `is_ge`/`is_clear_winner`), not new collisions. Adversarial stayed above baseline.
    The kill-gate rule names paraphrase/adversarial specifically and neither dip is
    large enough on its own to trigger it, but **direct has now dipped for two
    checkpoints running** (0.9363, then 0.9255) — worth a closer look next checkpoint
    before assuming this one is noise too.
  - Checkpoint 10 (`checkpoint-10-fractions-m1-complete`, 163 cells, fractions — GSM8K M1
    5/5, the campaign's last authored pack): P@1 direct 0.92 / paraphrase 0.40 /
    adversarial 0.41. **Direct's third consecutive decline** (0.9363 → 0.9255 → 0.9181)
    and **paraphrase's first measurable drop below baseline** (0.4016 vs 0.4247, ~2.3
    points — larger than the earlier "within noise" deltas of ~0.005). Of 6 flipped
    cases, 4 lost hit@1 and 2 gained it; of the 4 losses, 3 are directly attributable to
    this pack: `frac_sub`/`frac_cmp`/`frac_add`'s own summaries lead with generic
    arithmetic verbs ("subtract," "compare," "add") that now outrank `sub_sat`, `eq`, and
    `same_unit_check` on their own established queries (`sub_sat-para-1`, `eq-adv-1`,
    `same_unit_check-para-1`). The 4th (`fits_u16-direct-1` losing to `abs_i16`) is
    unrelated drift from the earlier signed-deltas pack. Adversarial held steady above
    baseline. **This is the first checkpoint where the kill-gate's literal condition
    (paraphrase or adversarial dropping meaningfully from baseline) arguably applies** —
    paraphrase's drop is real and attributable, not a single-query coincidence, and it
    rides on top of a genuine multi-checkpoint direct decline. Flagged to the user as a
    decision point rather than auto-continuing past it (see the session's response for
    the resolution reached).

Re-record after a change that claims to move one of these (library growth, diagnostic
rewrites, index changes) and compare in the diff — drift is the signal.
