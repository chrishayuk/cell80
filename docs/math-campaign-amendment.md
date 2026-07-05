# PlanFix → M3 — the amended campaign plan

*Status: **registered, 2026-07-05** — supersedes the extraction leg of
`docs/math-campaign-spec.md` while keeping its hypotheses, gates, and honest-limits
discipline intact. Written the same day `experiments/planfix` landed, because the spec and
the evidence now disagree and the spec should lose: M2's plan-IR extraction is no longer
the best-known configuration on our own measurements (JSON 10–75% vs direct-Rust 65–95%
across the weak/strong pair, `format_bakeoff.py` / `rust_bakeoff.py`). This plan registers
the replacement before M3 spends real compute.*

**One sentence:** move canonicalization into the compiler, close the two mechanical
escalations, validate the headline model and the PAL baseline, then run the campaign with
code-form extraction, the cross-check gate, and `correct_via_solve` accounting everywhere.

---

## What is settled (inputs to this plan, not open questions)

1. **The substrate executes correct plans cleanly.** 123/123 hand-extracted smoke test;
   render/kill surfaces every mechanical failure as a named rejection, never a silent
   wrong number. (`m3_gsm8k_smoketest.rs`, pilot Part 1.)
2. **Plan-IR JSON is the wrong ask for small models.** 15% single-shot / 30% +repair on
   granite4.1:3b; four models produced four distinct malformed-JSON dialects; every
   malformation class is an artifact of flattening a DAG into tuples. (Pilot Parts 1–2.)
3. **Native tool-calling is worse than scripted extraction and adds a silent-bypass
   failure axis.** ~0% genuinely tool-verified; models self-compute unverified answers
   when a tool frustrates them. Gate on `correct_via_solve`, always. (Pilot Part 2.)
4. **Code-form extraction dominates.** JSON → arithmetic-AST → direct-Rust:
   qwen2.5:3b 10% → 70% → 65%; gemma4:e4b 75% → 95% → 95%. The weaker the model, the
   bigger the payoff. (`format_bakeoff.py`, `rust_bakeoff.py`.)
5. **The structured cross-check gate is safe and yields.** Inline-arithmetic vs
   library-composition, accept only on agreement: gemma4, 20 problems, 16 accepted,
   16/16 correct, 0 false positives, 4 escalations (2 mechanical, 2 genuine
   comprehension). Precision stayed 100% while recall scaled with model strength.
   (`full_crosscheck.py`, `escalation_analysis.py`.)
6. **The linker works with zero compiler edits.** Model names intent; search + arity
   matching resolves the verified cell (7/8); unresolved calls inline-recompile to one
   re-checkable cell. (`call_match.py`, `compose_link.py`.)
7. **Noun-copied identifiers poison precipitation.** Identical structure with different
   field names renders different Rust → different artifact hash → H-M3 undercounts real
   recurrence. Same-structure-must-hash-same is a hard requirement, not polish.
8. **Deterministic repair earns its keep; model repair does not.** Defer-division fixed
   4/8 captured failures with the model untouched (qwen 50%→70%); model repair@1 was 18%
   with byte-identical broken echoes. The repair budget goes to compiler passes.

---

## Milestones

### M2.5 — the canonicalization pass (rustz80, `syn` level)

*The centre of the plan. One pass, one place, both extraction paths converge through it.*

After parse, before hashing and codegen, a deterministic normalization pass over the
`syn::File` (`rustz80/src/canon.rs`). *Anchor-point correction (2026-07-06):* the
artifact hash covers the manifest's `source_hash`, which is computed over raw source
**text** (`cell80/src/cartridge.rs`), so the pass is invoked text→text at the top of
`Cartridge::compile` — canonicalizing only the AST fed to codegen would leave two
differently-spelled twins hashing differently and H-M3 unmeasurable. Two strengths:
`Light` (dialect normalizer only, byte-stable when nothing fires — the default, so
hand-authored library cells keep their hashes and their named-args ABI) and `Full`
(everything below — the compose/campaign path and the plan renderer):

- **Alpha-rename** every binding to canonical slots (`q0, q1, …` for declared quantities,
  `v0, v1, …` for derived values) **in dataflow order** — order of first use in the
  topologically sorted op sequence, not source order. Source names survive only as
  metadata (`{"source_name": "pencils", "slot": "q0"}`) carried in the plan/fact row,
  never in rendered Rust.
- **Canonical op ordering:** topological sort with a deterministic tie-break
  (op kind, then operand slot indices) so independent ops always serialize identically.
- **Constant normalization:** decimal → fraction (`0.9` → `9/10`), and the
  **defer-division rewrite** (flatten `*`/`/` chains to `(num, den)`, divide once at the
  end) moves here from the Python `ast` path, so direct-Rust extraction gets the
  precision fix too. Overflow from reorder hits checked arithmetic → escalate, as now.
- **Unit base-scale table**, fixed and versioned in the pass, not the prompt:
  `money → cents always · time → seconds · unknown nouns → count · rates →
  explicit numerator_per_denominator`. Scale factors applied deterministically and
  recorded as repair metadata (`{"kind": "unit_scaled", "from": "dollars", "factor": 100}`).
- **Identifier safety becomes structural:** natural words never become Rust identifiers,
  so the `final`-class keyword leak is impossible by construction rather than
  blocklist-patched. The blocklist stays as a belt-and-braces assertion.

**Acceptance tests (all must pass before M3):**

| test | assertion |
|---|---|
| pencils/notebooks | same structure, different nouns → **identical artifact hash** |
| slot-order stability | permuted-but-equivalent source op order → identical hash |
| dollars / $16.50 mixed | canonical plan is cents-only, factor recorded |
| reserved identifiers (`final`, `try`, `union`) | render cleanly via slots, no parse error |
| rate nouns | `numerator_per_denominator` unit emitted |
| unknown nouns (sheep, cups, GB) | `count` convention applied |
| defer-division parity | Python-`ast` path and direct-Rust path produce identical canonical plans for the same expression |

*Explicitly not built:* a Python pre-render repair membrane, JSON-Patch model repair, a
shape classifier for comparison problems. Comparison is a function call the linker
resolves (`max`, `is_gt`, `choose_best3`); model repair is a last-resort rung behind
every deterministic pass, used only when a typed rejection names a fix no pass covers,
and logged as such.

### M2.6 — width + dialect normalizer (the two mechanical escalations)

*Rescoped 2026-07-05 (review): the width fix is built as **compile-time constant
folding with static width inference** — the stronger form. Constant subexpressions
fold exactly (`88000/11` → `8000` at compile time, no runtime width to overflow);
what the folded constants require decides the lane statically; exact-division
violations on constants are typed compile errors, not runtime kills; and folded
constants canonicalize harder, so more structurally-identical plans hash identically.
Diagnostics are **typed** (`rustz80/src/diag.rs`): stable `E*` codes with a span-free
message and `suggested_fix`, the normalizer keyed off codes rather than string
matching, and repair rows carry the codes so the M3 taxonomy is a query, not a grep.*

- **Width-aware compilation:** default composed cells to `u32` return
  (`wide_default`), auto-widen when any literal or folded constant exceeds
  `u16::MAX` (`E0301`), and record `E0503 wide_call` so linker resolution prefers
  `_u32` overloads. Kills the row89 class (`88000/11` — correct plan, overflowed width).
- **Dialect normalizer** (same pass family as M2.5): trailing-`let` → tail expression
  (row93), strip statement macros, bind compound call-arguments to `let`, collapse
  redundant parens.
- **Library slice:** `u32` variants of the comparison/aggregate core
  (`min_u32`, `lcm_u32`, `is_gt_u32`), fraction/rational cells
  (`scale_frac`, `percent_of`, `ratio_part` — defer-division baked in), `mean2/3`,
  `sum3/4`, `clamp`. Through the admission gate like any other cells, with retrieval
  rows. Re-measure the retrieval curve after landing — the second slice already paid a
  measurable retrieval-curve cost and this one must be priced the same way.

**Prediction, registered now:** re-running `full_crosscheck.py` on gemma4 after M2.5+M2.6
lifts yield **80% → ≥90% at unchanged 100% precision** (the two mechanical escalations
recover; the two comprehension escalations correctly persist). If precision moves, stop:
a normalization pass that converts an escalation into an accepted-wrong is a defect in
the pass, and the pass gets reverted before anything else lands.

### M2.7 — decorrelate the gate (third derivation, different reader)

The 100%-precision result is empirical, not structural: both derivations share one model,
one reading, temp 0. A correlated misread ("10 more" → "×11" in *both* paths) would be
accepted wrong, and N=16 cannot exclude a several-percent rate of exactly that.

- Add a **third derivation with a decorrelated reader**: either a second model
  (granite reads, gemma composes) or a deterministic paraphrase-then-extract pass on the
  same model — the requirement is a different *reading*, not just a different encoding.
- **Acceptance rule, registered now:** 3-way agreement → accept; 2-of-3 majority →
  accept **and flag** (majority-accepted rows are reported separately in M3 so the
  precision claim can be audited at both strictness levels); no majority → escalate.
- This also recovers the row86/row101 class (one path right, one misread) that strict
  2-way agreement currently escalates.

### M2.8 — validation runs (cheap, blocking, before the campaign)

1. **Granite through the full pipeline.** `full_crosscheck.py` + the M2.5/M2.6 passes on
   granite4.1:3b, the 20-problem slice. The spec's headline model has never touched the
   new pipeline; gemma4's yield is not evidence about it, and the pilot showed failure
   dialects are strongly model-specific. *Registered hypothesis H-P2 below.*
2. **PAL-Python baseline script.** Same 20 problems, same models, extract-Python-and-run.
   Direct-Rust-to-cell is structurally PAL with a different executor; H-M2 cannot be
   deferred any longer, and this is the easiest script in the campaign.
3. **Battery discipline confirmed on the composed path:** counterfactual perturbation
   runs on all multi-derivation survivor sets *even when they agree* (the
   coincidental-agreement fix), now exercised against real model-composed cells.

### M2.9 — productionize (`cell80 compose`) + fact provenance

- **`cell80 compose <source>`** subcommand and MCP **`cell_compose`**: parse →
  canonicalize (M2.5) → link (search-resolve calls, inline, recompile) → compile → run,
  with the N-derivation gate as a first-class solve mode. The Python retry loop in
  `compose_link.py` becomes the reference implementation, not the product.
- **Fact provenance:** every *accepted* composed answer emits a verified fact row —
  artifact hash (post-canonicalization), canonical args, result, cycle count, derivation
  count, agreement level — into the `.facts` file. Accepted answers become re-verifiable
  procedural memory; this is the bridge from the gate to the fact-file thesis, and it is
  what makes M3's residue auditable rather than asserted.
- Scripted harness only. No native tool-calling anywhere in the campaign path (settled
  input 3). `cell_compose` exists for agents *outside* the campaign; the campaign drives
  the binary directly.

### M3 (amended registration) — the campaign

**Configurations:** {granite4.1:3b, qwen2.5:3b, gemma4:e4b} × {CoT, PAL-Python,
cells-crosscheck}. Frontier reference (CoT) retained for context only.

**Corpora:** GSM8K test (1,319) first; GSM-Symbolic perturbation suite second, on the
same problem identities, because H-M1 is the headline.

**Extraction:** code-form (direct restricted-Rust primary; arithmetic-AST as the second
derivation), scripted, temp 0. Plan IR remains the internal wire format of the arithmetic
path — demoted, not deleted.

**Accounting rules, non-negotiable:**
- `correct_via_solve` is the only correctness counter. A stated answer with no successful
  deterministic derivation behind it counts as *bypass*, reported as its own column.
- Precision reported at both strictness levels (unanimous vs majority-accepted).
- Every escalation classified: mechanical / comprehension / width / shape — the taxonomy
  from `escalation_analysis.py`, frozen before the run.
- Cost per verified answer = tokens + T-states + wall-clock, per configuration.
- Precipitation counted **only on post-canonicalization hashes** (H-M3 is unfalsifiable
  on raw hashes — settled input 7).

**Hypotheses (H-M1–H-M4 carried unchanged from `math-campaign-spec.md`, two added):**

- **H-M1 (robustness):** 3B+cells degrades ≤ half as much as 3B-CoT on GSM-Symbolic.
  *Kill:* indistinguishable degradation → fragility is reading, not arithmetic; banked
  negative, campaign narrows to the verifier role.
- **H-M2 (parity):** 3B+cells accuracy ≥ 3B+PAL-Python. *Expected marginal, stated now;*
  the differentiators are H-M1, cost, precision, and residue.
- **H-M3 (precipitation):** the schema curve bends — post-canonicalization hashes recur
  and are retrieved at a growing rate. *Kill:* everything compiles fresh even after
  canonicalization → the procedural-memory claim fails in its best domain; outranks any
  accuracy number.
- **H-M4 (cost):** cost per verified answer for 3B+cells beats every configuration that
  matches its accuracy.
- **H-P1 (precision at scale), new:** accepted-and-wrong < 1% at N=1,319 under the
  registered acceptance rule. *Kill:* correlated-misread acceptances above threshold →
  the 2-derivation gate is insufficient and the decorrelated third reader becomes
  mandatory, not optional, before any yield claim is published.
- **H-P2 (format advantage generalizes), new:** granite's code-form accepted-and-correct
  rate exceeds its own plan-IR-JSON rate (30% combined) by ≥2×. *Kill:* granite gains
  little from the format switch → the bakeoff result is a qwen/gemma idiosyncrasy, the
  headline configuration changes model, and that is stated openly.

### M4 — readout

Unchanged decision structure: extend (contest packs) / narrow (verifier-only) / bank.
One addition: the fact file and precipitated library exported from M3 are first-class
deliverables of the readout — the campaign's artifact is a library plus a `.facts` file,
not a score. MATH/AIME remain gated behind this readout.

---

## Sequencing

| gate | contents | blocks |
|---|---|---|
| **M2.5** | canonicalization pass in rustz80 + acceptance tests | M3 (H-M3 unmeasurable without it) |
| **M2.6** | width-aware u32 · dialect normalizer · u32/fraction/aggregate cells · registered yield prediction | M3 (known fixed-cost artifact otherwise) |
| **M2.7** | third derivation, decorrelated reader · registered acceptance rule | H-P1 audit path |
| **M2.8** | granite validation · PAL-Python baseline · battery-on-agreement confirmed | H-P2, H-M2 |
| **M2.9** | `cell80 compose` + `cell_compose` · fact provenance · cross-check as solve mode | M3 harness |
| **M3** | the campaign, amended registration above | M4 |
| **M4** | readout: extend / narrow / bank | — |

M2.5 and M2.6 are one worktree (same pass family). M2.7 and M2.8 can run in parallel with
M2.9. Nothing in M3 starts until every M2.5 acceptance test is green and the M2.6
prediction has been checked against the 20-slice.

**Status (2026-07-06, branch `feat/canonicalization-m25`):** M2.5 has landed —
`rustz80::canon` + `rustz80::diag`, plan renderer emits q/v slots natively
(blocklist demoted to impossibility-by-construction), `Cartridge::compile`
canonicalizes text-level, and **all seven acceptance tests are green**
(`cell80/tests/canon_acceptance.rs`, `rustz80/tests/canon.rs`). The M2.6
compiler half has landed (constant folding, static width inference, typed
diagnostics, dialect normalizer) and the library slice is covered — wave 4
supplied most of it; `sum4` and `scale_percent_u32` fill the last gaps
(258 admitted, 0 refused). Cross-language defer-division parity
(Python-`ast` arm vs direct-Rust) and the **registered M2.6 yield prediction
(gemma4 80% → ≥90% at unchanged 100% precision)** still need model runs —
they are the first two items of M2.8, not silently skipped. The retrieval
curve after the new slice has not been re-measured yet either.

**Status (2026-07-06, second slice):** the M2.9 harness half has landed —
`cell80 compose <dir> <src.rs>…` (`cell80/src/compose.rs`): Full
canonicalization, the `E0504`-cued link loop (search + arity + a measured
link-confidence floor of 0.6, lexical containment rescuing the `gcd3`→`gcd`
class — a nonsense call name is a typed refusal, never a silent guess),
the registered N-derivation agreement gate (`unanimous` / `majority`+flag /
`escalate`) as a first-class solve mode, `--facts` provenance for accepted
answers, and the precipitation counter on composed schemas. Two deliberate
deltas from the original M2.9 text: composed cells widen **when constants
demand it** (the fold detection) rather than unconditionally — the library's
inlinable free-fns are u16, so a blanket-wide lane would unlink every call;
and the MCP `cell_compose` tool (Python surface) is deferred until the
campaign harness needs it. Free-fn `_u32` siblings of the state-cell wide
family are the natural next library slice (legal since two-u32-params).

---

## Out of scope, stated so nobody re-litigates it mid-campaign

- **Native tool-calling** in any campaign path (settled input 3; `cell_compose` exists
  for agent use outside the campaign).
- **A Python repair membrane / JSON-Patch model repair** (settled input 8; deterministic
  passes own repair, model repair is a logged last-resort rung).
- **A shape classifier for comparison problems** (the linker resolves comparison as
  function calls; typed `unsupported_shape` escalation remains only for shapes the
  dialect genuinely cannot express).
- **IR extension** (comparison ops etc.) — the escalation-rate data from M3 decides
  whether anything earns dialect entry, per the existing spec's discipline.
- **i32, MATH/AIME, bespoke per-model schemas** — all gated exactly as before.

## Honest limits

Cells fix arithmetic, not reading. The dominant residual failure class after the format
switch is comprehension, and no canonicalization pass touches it — the gate's job is to
*escalate* on it, and two of the four capstone escalations are that property working.
The 100%-precision figure is an N=16 result from one model with correlated derivations;
H-P1 exists because that number is currently a promise, not a finding. Granite — the
model this campaign is nominally about — has not yet produced a single accepted answer
through the new pipeline. And the precipitation claim, the most cell80-specific claim in
the campaign, is measurable only after M2.5 lands; until then H-M3 has never actually
been tested against anything.
