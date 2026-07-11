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
  - Checkpoint 11 (`checkpoint-11-retrieval-complexity-tiebreak`, 163 cells, **no new
    cells** — a retrieval fix in response to checkpoint 10's kill-gate flag, per the
    user's "pause growth, fix retrieval first" call): P@1 direct 0.91 / paraphrase 0.41 /
    adversarial 0.47. Diagnosed all 3 fractions-attributable losses from checkpoint 10
    directly (`cargo run --example retrieval_compare`-adjacent scratch harness): each was
    a simple 2-arg free-fn (`sub_sat`, `eq`, `same_unit_check`) losing to a 4-6-field
    fraction state cell that happened to share one core verb — a genuine, measurable
    same-verb-different-shape confound, not noise. Fix: `TfidfIndex::search` (the *live*
    path — `CellHost`/CLI/MCP/`cell-eval`'s `lib.search` all route through it) now breaks
    near-ties toward the structurally simpler cell (`rank_key = cosine / (1 + 0.05 · max(0,
    complexity − 2))`, complexity = free-fn param count or state-cell field count) — the
    same length-normalisation instinct BM25 applies to document length, applied to a
    cell's shape instead. Swept γ 0.0–0.3 on the full 327-query set before picking 0.05,
    the best overall point and the only one positive on every split but direct.
    Deliberately **not** applied to `TfidfIndex::scored`'s exposed magnitude — that value
    feeds `cell-eval`'s tiered-retrieval margin gate (`tiers.py`), calibrated against raw
    tf-idf cosine; rescaling it would silently drift an already-tuned θ (0.14 for
    `cell-potion`, etc.) without re-running that calibration, so only `search`'s ranking
    *order* changed, never the number. Result: adversarial jumped well above checkpoint
    1's baseline (0.4706 vs 0.3939, +0.09), paraphrase recovered part of checkpoint 10's
    drop (0.4098 vs 0.4016, still ~1.5 points under the 0.4247 baseline — a partial, not
    complete, recovery), direct ticked down a further 0.6 points (0.9123, continuing a
    now-four-checkpoint decline whose *rate* has been steadily slowing each checkpoint —
    consistent with a natural denominator effect of a growing corpus, not something this
    fix caused on its own). Net: real, validated, honestly-partial progress on the
    kill-gate's literal concern (paraphrase/adversarial); reported to the user as such
    rather than declared fully resolved.
  - Checkpoint 12 (`checkpoint-12-retrieval-tag-audit`, 163 cells, **no new cells** — the
    user asked to keep pushing on retrieval after checkpoint 11's partial recovery): P@1
    direct 0.92 / paraphrase 0.46 / adversarial 0.50. Dumped every current miss (direct,
    paraphrase, adversarial — a small scratch harness over `TfidfIndex`, not committed)
    and found a genuinely different, more widespread root cause than checkpoint 11's:
    several of the library's **oldest cells** (`gcd`, `min`, `max`, `chebyshev`,
    `pack_u8`, `same_unit_check`) were authored with much sparser tags than the richer,
    synonym-heavy convention later packs settled into, so newer siblings with fuller
    vocabulary (`gcd3`, `min3`, `max3`, `manhattan`) out-ranked them on their own queries.
    Six targeted, low-risk tag/wording additions, each verified directly against
    `examples/retrieval_compare` before and after (a seventh, adding "minus/subtract/
    magnitude" to `abs_diff`, measurably *regressed* adversarial and was reverted —
    verify every change, don't assume more vocabulary is strictly better):
    - `gcd`: added `divisor, common, factor, highest` (was missing all of them, despite
      `gcd3` having them — direct query "greatest common divisor" lost to `gcd3`).
    - `min`/`max`: added `smaller, smallest, least, lesser` / `larger, bigger, greatest,
      greater` (paraphrase queries using these synonyms lost to `min3`/`max3`/`is_lt`).
    - `chebyshev`: added `larger, max, maximum, axis` — its own definition *is*
      `max(|dx|,|dy|)`, but the tags never said so.
    - `pack_u8`: added `high, low, hi, lo` — the summary uses the abbreviations `hi`/`lo`
      as variable names, which don't tokenize to the words a query naturally uses.
    - `same_unit_check`: replaced a long inline enumeration of all 8 dimension codes
      (`0=count,1=money,...`) with a pointer to `docs/library-growth.md`, and added
      `match, mismatch` — the enumeration was diluting the doc's own normalised vector on
      the words that actually distinguish it from `unit_cancel_check`, and the summary
      never used the word "match" despite that being the cell's whole point.
    Result: **paraphrase (0.459) and adversarial (0.50) are both now above checkpoint
    1's baseline for the first time since checkpoint 7**, and **direct (0.9181) recovered
    fully to checkpoint 10's pre-fix level**, ending the four-checkpoint decline.
    Overall P@1 (0.7034) now exceeds checkpoint 1's own overall (0.6974) despite the
    library growing 114→163 cells. The kill-gate concern raised at checkpoint 10 is
    resolved, not just mitigated.
  - Checkpoints 13-16 (203→209 cells: the GSM8K math campaign's second slice + retrieval
    recovery, third slice, fourth slice) are recorded in `library-scale-curve.json` but
    were narrated in `docs/library-growth.md`'s own prose (the "second slice"/"third
    slice"/"fourth slice" pack notes) rather than backfilled here — not duplicating that
    account.
  - **Checkpoint 17 (`checkpoint-17-workflow-batch`, 395 cells, commit `3e757f9`,
    2026-07-11) — recorded after a long gap.** No checkpoint was taken between here and
    checkpoint 16 despite 186 cells landing (the math-server mining campaign, waves 6-14,
    plus two ecosystem-mining/family-expansion batches) — a real process gap, not a
    deliberate skip. P@1 direct 0.8202 / paraphrase 0.3887 / adversarial 0.4167 (797 cases).
    Against checkpoint 1's baseline (114 cells): paraphrase essentially flat (0.4247 →
    0.3887, −3.6 pts), **adversarial still above it** (0.3939 → 0.4167, +2.3 pts) — after
    the library grew 3.5×. The kill-gate does not trip. Full account, including the
    checkpoint-16→17 adversarial dip (−8.3 pts, likely the same same-shape-sibling effect
    checkpoint 12 already diagnosed as text-search-unfixable) in `docs/library-growth.md`'s
    "Checkpoint 17" note.
  - **Checkpoint 18 (`checkpoint-18-workflow-round2`, 500 cells, commit `6ad158a`,
    2026-07-11) — a second Workflow batch, checked immediately rather than left to drift.**
    P@1 direct 0.8082 / paraphrase 0.3891 / adversarial 0.4444 (1007 cases). Against
    checkpoint 17 (395 cells): paraphrase flat, **adversarial recovered the checkpoint-17 dip
    and then some** (0.4167 → 0.4444, +2.8 pts). Against checkpoint 1's baseline (114
    cells): paraphrase still essentially flat (0.4247 → 0.3891, −3.6 pts), and
    **adversarial is now clearly above it** (0.3939 → 0.4444, +5.1 pts) — after the library
    grew 4.4× over the session. The kill-gate has not tripped across either of the two
    Workflow batches.
  - **Checkpoint 19 (`checkpoint-19-workflow-round3`, 653 cells, commit `1d50f03`,
    2026-07-11) — the kill-gate trips for real.** P@1 direct 0.8087 / paraphrase **0.3736**
    / adversarial 0.4167 (1313 cases). Paraphrase is 5.1 points below checkpoint 1's
    0.4247 baseline — more than double the ~2.3-point drop that triggered the original
    checkpoint-10 pause. Diagnosed rather than patched blind: of 386 cells appearing as a
    paraphrase/adversarial miss, only 11 have genuinely sparse tags (below the library's
    median of 9); the other 375 are same-shape-sibling saturation (`gcd` vs `gcd3`/
    `gcd_u32`, etc.) — the class this project has repeatedly found is not fixable by
    wording, and three rounds of deliberately building missing siblings is exactly what
    grows it. Not launched past — flagged for a decision, matching the checkpoint-10
    precedent.
  - **Checkpoint 20 (`checkpoint-20-tag-recovery`, 653 cells, commit `54924f6`,
    2026-07-11) — the fix, measured.** Ten cells with genuinely sparse tags (`abs_diff`,
    `manhattan`, `weighted_sum`, `range_check`, `avg2`, `days_in_month`, `bcd_encode`,
    `dot2`, `mod_u32`, `q_sqrt`) got targeted additions, each aimed at a specific missed
    query. Result: direct 0.8042 (−0.45pt, noise), **paraphrase 0.3866 (+1.3pt, ~25% of
    the drop recovered)**, **adversarial 0.5000 (+8.3pt)**. 13 of 16 newly-fixed cases were
    the targeted cells' own previously-missed queries (confirming the fix worked as
    intended); the 8 regressions elsewhere were all inspected and are benign same-shape
    reshuffling (a cell now ranking #2 behind its own u32/i16 sibling instead of #1). A
    partial recovery, the same honest shape as checkpoint 11 — the dominant remaining cause
    needs the structural lever this project has already named and not yet built
    (behavioural I/O-example routing, or a type-led index that discriminates on structural
    shape).
  - **Checkpoint 21 (`retrieval-examples-653cells-2026-07-11.json`, 653 cells,
    2026-07-11) — the structural lever, built and measured: F2 PASSES.** Behavioural
    I/O-example routing fused into the primary search path (`CellHost::
    search_with_examples`: behaviour ranks, plain-search order breaks ties, zero-match
    cells demote instead of dropping) plus a generated example sidecar
    (`datasets/retrieval-examples.jsonl`, ≤3 plausibly-user-authorable examples per case,
    98.5% coverage, `co_match` honesty metadata). **Probe-equipped paraphrase P@1 0.859**
    vs the 0.39 plain baseline on the same 603-row equipped subset — the WS-F F2 gate
    (≥ 0.80) clears with headroom; adversarial 0.47 → 0.89, direct 0.81 → 0.95, overall
    deployed 0.90. Zero per-query regressions (guaranteed: expected reproduces its own
    examples, ties preserve text order — fused rank ≤ plain rank always). The honest
    residue: 85 paraphrase misses, 45 carrying residual `co_match` — the class examples
    cannot separate *by construction* (`min(a,b) ≡ median3(a,b,0)` under register
    zero-fill; predicate families where dozens of cells return 1 on (1,1)); the rest lose
    the text tiebreak to co-equal matchers outside the modelled sibling pool. Caveat kept
    in view: this measures example-CARRYING requests — text-only paraphrase (checkpoint
    20's 0.3866) is unchanged and stays the open problem for text-side levers.

  - **Checkpoint 22 (`retrieval-examples-arity-653cells-2026-07-11.json`, 653 cells,
    2026-07-11) — the arity tie-break dissolves the zero-fill class.** Checkpoint 21's
    residue analysis showed `min(a,b) ≡ median3(a,b,0)`-style co-matches are an artifact
    of register zero-fill, not real behavioural identity — the manifest knows the arity.
    The fused ranking now breaks behavioural ties by declared-arity match before text
    (order-only; the zero-hit tail keeps pure text order so garbage examples still
    degrade to plain search; the expected cell can't lose the tie-break when the
    examples fit its own signature, so fused-rank ≤ plain-rank still holds — verified,
    zero regressions). Result: **equipped paraphrase 0.859 → 0.879, adversarial
    0.89 → 0.92, direct 0.95 → 0.96, deployed overall 0.90 → 0.91**; paraphrase misses
    85 → 73, of which co_match-attributed 45 → 33. The remaining 33 are behaviourally
    indistinguishable *at matching arity* (predicate families agreeing on the supplied
    probes) — the class that needs active disambiguation, not better ranking.
  - **Naming discipline: checkpoints 21/22 measure ORACLE-EQUIPPED behavioural
    retrieval** — examples derived from the expected cell by `gen-examples`, i.e. the
    router's discrimination capacity given correct examples. They do NOT measure whether
    an agent can author useful probes from a natural-language request; that end-to-end
    benchmark (request → model-authored examples → fused retrieval, tracking example
    validity/informativeness/correlated failure) is registered as owed work below. Do
    not read 0.88/0.91 as "arbitrary text queries are solved."

## Round-4 re-registration (2026-07-11) — the growth gate splits into two lanes

Registered on user decision after checkpoints 19–22. Rationale: the original kill gate
("does the library stay searchable as it grows?") conflated two lanes that now have
different answers. The deployed retrieval contract includes optional executable
behavioural probes on every surface (CLI/MCP/py), so fused retrieval becomes the
blocking gate; text-only remains a tracked product weakness, watched — not a veto.
The paper trail is this entry; the change is dated, motivated, and its floors were set
from pre-registered targets, not fitted to the measurements.

**Blocking growth gate (fused, oracle-equipped)** — enforced in
`cell-eval/tests/test_retrieval_examples.py`:

    equipped coverage           >= 0.95   (measured 0.985)
    equipped paraphrase P@1     >= 0.82   (measured 0.879)
    equipped adversarial P@1    >= 0.82   (measured 0.92)
    deployed overall P@1        >= 0.88   (measured 0.91)
    monotone rank regressions    = 0      (fused rank <= plain rank, every equipped case)

**Text-only fallback watch** — reported every checkpoint, two enforced floors:

    text-only direct P@1        >= 0.80   (test_retrieval.py, unchanged)
    checkpoint-1 cohort ratchet >= 0.4247 (test_retrieval.py::test_checkpoint1_cohort_ratchet)
    text-only paraphrase/adversarial (full mix): report only

**The dilution analysis that priced the ratchet (2026-07-11):** re-scoring the FIXED
228-query checkpoint-1 cohort against the 653-cell library: paraphrase **0.4247 → 0.4795**
(+5.5pt), adversarial **0.3939 → 0.5152** (+12.1pt), direct 0.9426 → 0.8279 (the known
benign sibling-shading class — `manhattan` behind `manhattan_wide`, etc.). Text-only
retrieval on fixed queries *improved* through 5.7× growth; the library-wide paraphrase
decline (0.4247 → 0.3866) is **composition** — new packs deliberately add sibling-dense,
harder queries — not erosion. Hence a fixed-cohort ratchet at the origin value: it fires
only on genuine degradation of existing retrievability, and cannot be gamed by the query
mix. (Direct on the cohort is intentionally not ratcheted separately — the library-wide
direct floor already covers it and the shading class is documented benign.)

**Round-4 conditions (registered with the resumption):**
1. **Example-equipped admission** — every admitted cell ships probe examples; the ≥0.95
   equipped-coverage floor is an admission invariant, not a happy accident.
2. **co_match density is a watched number per batch** — growth manufactures exactly the
   ambiguity class the residue is made of (predicate families, low-arity coincidences).

**First contact: Finance80 Wave 1 (2026-07-11, 653→697 cells, +136 eval rows).** The gate
worked exactly as registered — the wave landed without a sidecar regeneration and coverage
fell to 0.91, failing the blocking gate. Fixing forward surfaced two real defects:
(1) a **host bug** — `Runner::reset_for` (the pool-recycle path) never stamped the resident
kernel bank, so any `kernel_bank: on` cell recycled onto a bus born under a non-bank cell
ran into zeroed 0xC000 and died on `cycle_budget`; every warm host (MCP included) was
affected, found because the generator loads hundreds of cells before the finance group
(fix + regression test `cell80/tests/pool_bank.rs`); (2) a **generator gap** — f32 state
fields were excluded as "not user-authorable", but cells are deterministic softfloat, so an
f32 example matches bit-exactly; the sidecar now carries bit patterns whose information
content is ordinary decimals (rate=0.05), integer-only layouts byte-identical to before.
Post-fix at 697 cells: **coverage 0.984, equipped paraphrase 0.88, adversarial 0.93,
direct 0.95, deployed 0.92** — the f32-dense, deliberately-confusable finance families did
not dent the fused lane on their first wave. Residual unequipped: 23 rows whose cells'
validity domains reject the whole probe pool (iterative/date cells) — the active-
disambiguation / property-example work below is their lever.

**The agent-authored-example benchmark (2026-07-11, `cell-eval authored`) — first
result: the lane FAILS at 3B, and the failure has a deployable mitigation.**
The eval runs the deployed loop (request → model authors 1–3 I/O probes, never shown
the target → fused retrieval) over the schema-free-authorable population (value cells,
arity 1–3: 474 of 1,445 cases, 32.8% — state cells need the search→inspect→author
two-step, registered below). granite4.1:3b
(`authored-granite3b-697cells-2026-07-11.json`): well-formed 0.80 but **valid 0.35**
(the model cannot compute its own outputs), authored P@1 **0.45** — *worse than plain
text on direct* (0.51 vs 0.82), because invalid examples actively demote the right
cell — **false_unique_rate 0.095** (one query in ten confidently pinned to the wrong
cell), correlated failure 0.52. Read against oracle 0.96 on the same cases: the router
is fine; **test-authoring correctness is the bottleneck**, exactly as the registration
anticipated.
**The junk-guard analysis** (same file's cases, `full-match` = some cell reproduces
every authored example — a query-time signal needing no ground truth): the guard
provably never discards valid sets (valid-without-full-match = 0), filters 56% of
invalid ones, and flips the hard splits positive — guarded paraphrase 0.42→**0.49**,
adversarial 0.44→**0.52** — while direct still bleeds (0.82→0.72) from the 94
invalid-but-full-match sets (wrong examples some other cell satisfies). The deployable
composition drops out: **answer high-text-margin queries from text (the existing
calibrated tier gate), spend guarded authored examples only on the escalated residue.**
Equivalence-aware metrics landed with the benchmark: on the oracle eval at 697 cells,
paraphrase strict 0.880 → **0.911 equivalence-aware** (top hit is a recorded co_match
sibling — behaviourally right, label-wrong); miss-outside-known-class 8.9%.
Scaling axis (does validity improve with model capability?): gemma4:e4b run pending —
same command, banked when complete.

**Owed work (registered, in rough order of leverage):**
- *Schema-aware authoring (two-step lane)* — search → inspect → author field examples
  for state cells (67% of the eval population is currently outside the schema-free lane).
- *Margin-gated authored retrieval* — the tier-gate composition above, measured.
- *Active disambiguation* — when the router returns a behavioural equivalence class,
  execute candidates until they diverge and surface the distinguishing input ("these 4
  agree on your examples; they differ on (3,1) — which output do you want?") instead of
  silently picking by text. Product feature on MCP; self-resolvable by the model in WS-H.
- *Equivalence-aware metrics* — split P@1 into unique-exact / expected-in-equivalence-class /
  actually-wrong; report ambiguity_rate and false_unique_rate (target 0: never claim a
  unique answer the examples don't justify).
- *Scaling checkpoints* at 1,000 / 2,000 / 5,000 cells, each reporting both lanes plus
  ambiguity-class rate and probe cost.

Re-record after a change that claims to move one of these (library growth, diagnostic
rewrites, index changes) and compare in the diff — drift is the signal.
