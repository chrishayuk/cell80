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

### M3 registration update (2026-07-07 — draft for sign-off before compute)

Everything M2.5–M2.9 measured feeds these deltas to the original M3 registration:

**Configurations.** {granite4.1:3b, qwen2.5:3b, gemma4:e4b} × {CoT, PAL-Python,
cells-3way} as registered, **plus a registered ensemble column**: weak composer ×
gemma reader (granite×reader, qwen×reader). Evidence: granite 35%→75% and qwen
15%→65%, both at 100% precision with zero genuine escalations on the slice — the
reader costs one inline generation and both weak models land on their composed-arm
ceiling. The ensemble is reported as its own configuration, never as the weak
model's solo number.

**H-P2 verdict, stated openly (kill clause applied).** granite-solo code-form
accepted-and-correct reached 45% vs its 30% plan-IR combined — 1.5×, under the
registered ≥2× bar. Per the registered consequence the headline configuration
changes: **gemma4 carries H-M2/H-P1; granite remains as the weak-model datapoint
and ensemble composer.**

**Accounting deltas (all landed and replay-verified).**
- Gate outcomes are the frozen vocabulary: `unanimous` / `majority` (flagged,
  audited separately) / `escalate` / `degenerate_zero` (zero-guard) /
  `battery_escalate` / `single`. Precision reported at both strictness bands.
- **Battery certificates required** for every multi-derivation accept: common
  lifted quantities perturbed, agreement must survive; perturbation counts and
  skipped values reported. (Priced: it correctly killed one exact-division-
  coincidence agreement on the slice.)
- **Checked emission everywhere on the compose path** — silent wrap is closed;
  overflow/negative escalate (`needs_wider_math`), matching plan-solve.
- **Precipitation (H-M3) counts post-canonicalization schema recurrence at the
  instance level** — lifting makes same-structure-different-numbers one artifact,
  so the schema curve is measured across problems, not just noun spellings.
- The E-code repair taxonomy as of `E0102`–`E0505` is frozen for the campaign.

**Phasing.** (1) gemma cells-3way at N=1,319 with CoT and PAL baselines — the
H-M2/H-P1 flagship; (2) GSM-Symbolic on the same identities (H-M1, the headline);
(3) weak-model solo + ensemble configs. The runner must checkpoint/resume
(~10k+ local generations), export facts, and capture tokens/T-states/wall-clock
per the original accounting.
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
campaign harness needs it. ~~Free-fn `_u32` siblings of the state-cell wide
family are the natural next library slice (legal since two-u32-params).~~
**Correction (2026-07-06):** checked directly — `mod_add_u32`/`mod_sub_u32`/
`mod_mul_u32` need *three* u32 params (`a`, `b`, `m`), and the Z80 calling
convention caps a function at two (the first rides `HL:DE`, a second the
stack; a third has nowhere to go — confirmed by an actual failed compile,
not inferred). "Two-u32-params legal" only clears the way for genuinely
2-param wide free-fns; the state-cell mod family can't cross to free-fn form
this way. See the mod-space rewrite note below for how the modulus threads
through without ever needing a 3-arg function.

**Status (2026-07-06, mod-space rewrite):** the first of the four structural
AIME/MATH gaps from the post-M2.9 gap analysis is landed — `canon.rs` Full
mode now recognizes a `<chain> % m` tail (`Node::Rem` at the root) where `m`
is a leaf (param or constant, so its value is available before any chain op —
no ordering hazard) and the chain feeding it is pure `Sum`/division-free
`MulDiv` (no `Call`, no nested `Rem`, no fractional constant), and rewrites
each step to reduce-combine-reduce mod `m` instead of computing the whole
chain wide and reducing once at the end. This is the AIME "finishing move"
(`... % 1000`) done from the start. Two things this buys, both proven
end-to-end against an independently-computed expected value
(`cell80/tests/mod_space_rewrite.rs`): the wide lane's `+`/`-`/`*` are
**unchecked**, wrapping ops (the canonicalizer never auto-inserts
`mul_checked_u32` et al. into ordinary arithmetic), so the naive
wide-then-mod path doesn't escalate on a real overflow — it silently wraps
mod 2^32 and reduces *that*, a different (wrong) residue whenever `m` isn't a
power of two, with no signal at all. The rewrite is exact in both directions:
a product chain whose true value vastly exceeds u32 (verified: three u16
params near 65535, true product ~2×10^14, naive wraps to a plausible-looking
wrong answer), and a chain that goes negative mid-computation (verified:
`(a - b + c) % 7` with `a < b`) where the naive wrap-then-mod also misses.
No new library cells or prelude kernels: each step reduces via `%` then
combines via the *existing* 2-arg `add_checked_u32`/`mul_checked_u32`
kernels (the correction above is why — a 3-arg shared kernel isn't
compilable), so the rewrite is pure canon-level codegen, typed as `E0206
mod_space_rewrite` when it fires. Falls back to the unchanged existing
emission (byte-identical) for division-containing chains, non-leaf moduli,
and anything with a nested call or remainder — all covered by tests.
Remaining three structural gaps (answer-format contract, inverse-solve,
sign convention) are unchanged by this slice.

**M2.8 item 1 result (2026-07-06, `experiments/planfix/crosscheck_m26_results.txt`
via `crosscheck_m26.py` — Python `autofix()` deleted, all repair in-compiler):**

- **gemma4: the registered yield prediction FAILED — banked as registered.**
  Yield 75% (15/20), vs predicted ≥90% and the pilot's 80%. **Precision held at
  100% (15/15, accepted-and-wrong = 0)** — the safety half of the prediction, and
  the revert trigger, did *not* fire. Reading of the miss: (1) temp-0 outputs
  drifted between runs, so the failure set is not the pilot's — row93 is now an
  unparseable source (`E0501`, which no normalizer can fix), and row94 is a new
  genuine `E0302 inexact_const_division`; (2) row89's width fix recovered only
  one derivation, and a one-sided recovery still escalates under a 2-way gate —
  the strongest evidence yet that **M2.7's third derivation is load-bearing**:
  4 of gemma's 5 escalations are recoverable (one path correct) and would accept
  under the registered 2-of-3 rule. E-codes: `E0505`×19, `E0301`×1, `E0203`×1.
- **granite (H-P2 preview): 35% accepted-and-correct (7/20) at 100% precision**
  vs its plan-IR 30% combined — a gain, but nowhere near the registered ≥2×.
  Failure mass is dialect (`E0502`×11: method-receiver style, assignments, parse).
  If the full M2.8 slice confirms this, H-P2's kill clause applies: the headline
  model changes, stated openly.
- **qwen2.5: collapsed to 1/20** with a systematic `E0501 expected ;` dialect
  (9 rows) — a model-specific spelling the pilot's bakeoff path never surfaced.
  Needs a captured-source diagnosis before any conclusion; possibly one cheap
  normalizer rule, possibly genuinely unparseable.
- Cross-model precision: **23 accepts, 0 wrong across all three models.** The
  gate's safety property survived the pipeline migration everywhere.

**Registered amendments (2026-07-06, user sign-off):**
1. **Zero-guard** — an agreed answer of `0` never accepts; it escalates as
   `degenerate_zero`. Rationale: `0` is the collapse value of broken derivations
   (granite's verify-not-compute else-arms); two unrelated broken programs agreeing
   on `0` is coincidence, not consensus. Counterfactually verified over every
   captured run (all models, both gates): removes the campaign's only
   accepted-and-wrong at **zero yield cost**. A legitimate zero answer (rare to
   nonexistent in GSM8K) escalates, which is the honest failure mode.
2. **Numeric method-call → kernel rewrite** (`E0205 method_to_kernel`):
   `a.max(b)` → `imax(a, b)`, `.min` → `imin`, `.abs_diff` → `iabs_diff` — the
   prelude kernels that already exist. Deterministic, semantics-preserving at u16,
   typed, recorded. Prices at +1–2 granite rows on the 20-slice.

**Registered amendment 3 (2026-07-07): the verify-`if` → computed-side rewrite
(`E0207 verify_rewrite`).** The shape `if <expr> == <literal> { <literal> } else
{ 0 }` — granite's signature verify-not-compute dialect — rewrites to `<expr>`:
the model *stated* the literal, but the comparison contains a real derivation, and
the rewrite returns what the arithmetic computes instead of the guess-or-zero.
Output-changing by design (a failed self-check now yields the computed value, not
the degenerate 0), which is why it carries its own registration and a
captured-source replay precision check: **if any configuration's accepted-wrong
moves off zero under replay, the rewrite reverts.** The zero-else arm is required
— it is what makes the else side contentless.

**Registered amendments 4–5 (2026-07-07): width belongs to the compiler.**
Asking a 3B model to manage Rust integer widths is asking too much — the width
errors it makes (`88000u16`, cargo-cult `as u16` mid-chain) are bookkeeping noise,
not arithmetic intent, and the checked lane already owns overflow honestly.
- **`E0208 suffix_normalized`** (Full mode): integer-literal width suffixes are
  advisory — stripped on parse, canonical suffixes re-emitted by the lane rules.
  An *impossible* suffix (value exceeds its own type, `88000u16`) is named in the
  repair row instead of dying as a parse-adjacent error; this reaches light-fallback
  fns inside a Full-mode source too.
- **`E0209 narrowing_dropped`** (checked/campaign lane only): a model-written
  `as u16` inside arithmetic is dropped — in the wide checked lane a mid-chain
  truncation destroys information the kernels are protecting, and is almost always
  the model fighting the type checker rather than meaning "reduce mod 65536".
  Output-changing by design, so: replay precision check across every captured
  configuration; **any accepted-wrong movement reverts it.** Hand-written sources
  (Light mode, plain Full without `checked`) keep real truncation semantics — the
  dialect and its rustc oracle are untouched.

**Registered amendment 6 (2026-07-07): `then`-sugar desugaring (`E0210
then_desugared`).** `if <cond> then <a> else <b>` — a non-Rust conditional granite
emits — desugars textually (comment-safe, before parsing) to `if <cond> { <a> }
else { <b> }`, coercing a `!`/`panic!()` else-arm to `0`, so the verify shape
reaches `E0207` instead of dying at `E0501 parse`. `then` never appears in valid
Rust, so the pass is byte-identical on well-formed sources. Replay: granite solo
9→10/20 (row104), granite×reader row104 majority→unanimous, 0 accepted-wrong in
every configuration.

**Registered amendment 7 (2026-07-08, user sign-off): the battery-unverified
guard.** A **majority** (flagged-band) accept that the counterfactual battery
could not verify at all (zero perturbations ran — wide values are unliftable
under the u16 parameter ABI, so the battery is structurally blind exactly there),
with wide values in play (a source literal or the agreed answer exceeds
`u16::MAX`), escalates as `battery_unverified` instead of accepting. Rationale:
the flagged band's contract is *accepted agreements survived perturbation*; this
refuses to pretend an unverifiable agreement did. Unanimous accepts are exempt.
Found by the two-weak-model ensemble experiment (granite composes × qwen reads
composed): both weak models made the same canonical ratio misreading of row89
("10 times as many" → divide by 10), agreeing on 79200 — a **correlated
misreading**, which no perturbation battery (u16 or u32; a recompile-perturb
variant was built, refuted by replay, and reverted — inexact-division kills the
exact perturbed values that would discriminate) can catch in general.
Counterfactually verified over every captured configuration (8 configs, 160
rows): removes the single accepted-and-wrong at **zero yield cost**. The general
defense against correlated misreading remains a decorrelated reader; this guard
closes the unverified-wide subclass and is honest about the rest.

**Registered amendments 8–10 (2026-07-08): the lifting tier hardened — the
mechanical backlog from the 8-shot runs, plus the stated-answer hole the fix
itself exposed.** All three verified together by full replay over every captured
configuration; the loop ran three iterations (each replay surfaced the next
defect) before converging at 0 accepted-wrong movement.

- **8 — `E0103 lift_cap_reached` (the lift cap).** Literal lifting stops at the
  calling convention's 3 register slots (HL/DE/BC); a 4th quantity stays a baked
  constant, reported, instead of the whole fn dying at lowering with "parameters
  exceed the 3 register slots" (the row124 class — equation-format sources lift
  many quantities).
- **9 — `E0211 call_to_wide_kernel`.** A call to `max`/`min`/`abs_diff` whose
  arguments include a wide *computed* value routes to the prelude's wide kernels
  (`imax_u32`/`imin_u32`/`iabs_diff_u32`) — the u16 library cell cannot take a
  u32 argument, and the wide `_u32` library siblings are deliberately state
  cells (the u32 test/CLI surface), not inlinable. Narrow-argument calls keep
  resolving to library cells, so retrieval/precipitation is untouched (the
  row97 class).
- **10 — the restatement guard.** A canonicalized tail that is a constant while
  quantities were lifted, or is itself a lifted literal parameter, is a *stated
  answer* wearing a cell's clothes (granite's `let total = 160 + 80 + 20`
  restatement style) — unfalsifiable or frozen under the battery, and a
  majority-confirmation backdoor. Such fns soft-fall to Light and run as
  written, with no lifted values, so the battery's skip semantics stay honest
  about them. This hole predates the cap — the slot-ceiling death was
  accidentally masking it. *Tried and reverted in the same loop:* restricting
  lifting to bare literals only — replay showed folded-init lifting is
  load-bearing (it rescues the row94 `E0302` false-kill class and gives the
  battery the reach behind its row117 exact-division-coincidence catch); the
  restriction cost three real rows to fix a case the guard's soft-fall plus the
  agreement set's own exclusion already handled honestly.

**Consolidated write-up of the full M2.5–M2.7 arc — builds, all three model runs,
the error analysis, and the proposed rule amendments awaiting registration:**
`experiments/planfix-m2-findings.md`. Headlines: gemma4 **19/20 at 100% precision
(both bands)** under the registered 2-of-3 gate; granite 35% → **70% at 100%
precision, zero genuine escalations** with the registered second-model reader;
49 accepts / 1 wrong campaign-wide, the 1 in the flagged band, diagnosed
(degenerate-zero agreement), with a zero-cost counterfactual fix (zero-guard)
proposed for registration.

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
