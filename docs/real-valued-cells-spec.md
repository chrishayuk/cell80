# Real-valued cells — the Q-format policy and the mined math packs

*Status: **draft for registration, 2026-07-06.** Revises the dialect's real-valued-number
policy and scopes the library growth mined from `chuk-mcp-math-server`'s 642-function
catalogue. Companion to `docs/10-dialect-semantics.md` (which this amends in one place),
`docs/math-campaign-spec.md` and `docs/planfix-to-m3-plan.md` (which this must not block),
and `docs/library-growth.md` (whose retrieval-curve discipline every wave below pays).*

**One sentence:** floats stay out of the dialect permanently — with the reason recorded —
and real-valued computation enters as unit-tagged, error-bounded, demand-gated Q-format
fixed point, grown by gap-filling the existing 209-cell library against the math-server
catalogue rather than authoring from a taxonomy.

---

## Part 1 — the policy revision

### 1.1 Floats: rejected permanently, and why (banked decision)

> **Amended 2026-07-07** by `docs/real-valued-cells-amendment.md` (the F-waves): the ban
> narrows from "no floats" to "no floats we don't own." Owned IEEE binary32 softfloat
> kernels enter the dialect; platform libm stays banned permanently — the rationale below
> is answered, not overruled. The amendment also opens Wave 3's representation choice
> (its §F2) while keeping the demand gate unchanged.

The current dialect doc lists floats under "out of the dialect (by design, not omission)"
without recording the design reason. The reason is the oracle, and it is worth banking so
nobody relitigates it with "but WASM has f32":

`check!`'s guarantee is that both compile targets agree with each other **and with
release-mode rustc on the host**. IEEE basic arithmetic is bit-specified, but the
transcendentals (`sin`, `cos`, `exp`, `log`, `pow` on reals) are not — rustc lowers them
to platform libm, and libm results differ across hosts. A cell whose reference behaviour
is "whatever this machine's libm returned" breaks three load-bearing properties at once:

1. **The differential oracle** can no longer assert anything portable.
2. **Content addressing** weakens — same source, same inputs, host-dependent facts.
3. **The fact file** quietly degrades from "memory you can't lie to" into "memory that
   was true on the machine that wrote it."

Add the substrate economics (soft-float on this core is hundreds of bytes and thousands
of cycles per op, inverting the tiny-artifact/µs-dispatch pitch) and the campaign
positioning (exact-by-default is precisely the claim over PAL-Python). Floats are not a
deferred feature; they are the escalation path, and the ladder already types them:
`halt(0xFF02)` = `needs_floats` → math-server rung → Python/PAL → frontier.

### 1.2 The distinction the old policy conflated

"No float type" and "no real-valued computation" are different claims, and the library
has already quietly demonstrated the difference: `q_mul`, `q_div`, `q_sqrt`, `q_lerp`,
and `q_sigmoid` are admitted Q8.8 cells today, with a working `//! scale: 8` manifest
convention. **Fixed point is integer arithmetic all the way down** — deterministic,
differentially testable against integer rustc with the oracle unmodified,
content-addressable, byte-cheap. The policy revision is therefore not "add a numeric
tier"; it is: *recognize the tier that exists, give it the three disciplines it is
missing, and grow it on demand.* The three missing disciplines:

1. **Scale in the type system**, not just the manifest (§1.3).
2. **Accuracy as a declared, tested contract** for approximate cells (§1.4).
3. **An exactness taxonomy** so Q never free-rides on the fractions' "exact" claim (§1.5).

### 1.3 Scale joins the unit system

The plan renderer's unit checker is an exponent vector over `[count, money, time,
distance]` — it type-flows *dimension* but not *scale* (`seconds` and `hours` are the
same dimension; Q8.8 and raw integers are indistinguishable). Extension:

- Every quantity carries a **scale** alongside its dimension: `0` (integer, default),
  `8` (Q8.8 in u16), `16` (Q16.16 in u32). Spelled in plan JSON as a unit suffix
  (`"scalar_q16"`, `"meters_q16"`) or an explicit `"scale"` field — renderer accepts
  both, canonicalization (M2.5) normalizes to one spelling before hashing.
- **Type-flow rules:** `add`/`sub` require equal scale (as they require equal
  dimension). `mul`/`div` of scaled quantities do **not** render as raw ops: the
  renderer routes them through the Q kernels (`q_mul`/`q_div` at the operands' scale),
  exactly as checked arithmetic routes through the checked kernels. A raw op on scaled
  operands is a render error, never a silent double-scaling.
- **Literals:** a decimal literal in a scaled position converts at compile time
  (`0.5 → 32768` at Q16.16), round-to-nearest, with the conversion error recorded in
  the plan metadata when nonzero. Compile-time and deterministic, so the oracle and the
  hash are untouched. (The *exact* path for `0.9`-shaped values remains the fraction
  cells + defer-division — see §1.5.)

This slots into the M2.5 canonicalization pass rather than competing with it: scale is
one more thing the pass normalizes, and the same-structure→same-hash requirement applies
across scale spellings.

### 1.4 The dual contract: determinism ≠ accuracy

Two guarantees, never conflated:

- **Determinism (the existing oracle, unchanged):** bit-exact agreement between both
  targets and integer release rustc, on every input tested. Q cells are integer
  programs; `check!` needs no modification.
- **Accuracy (new, for approximate cells only):** a manifest line
  `//! accuracy: |err| <= 2^-12 over [domain]` — asserted by a separate CI harness that
  sweeps the declared domain against an f64 reference computed at test time (f64 is fine
  *in the test harness*; it never enters a cell). The harness reports measured max
  error; measured > declared fails CI.

**Admission extension:** an approximate cell (any cell whose manifest declares an
`accuracy:` bound) must carry accuracy rows the way every cell carries retrieval rows —
no bound declared, or no sweep passing, no admission. Exact cells are unaffected.

### 1.5 The exactness taxonomy (manifest field, three values)

- `exact` — integer and fraction cells; results are the mathematical answer or an
  escalation. This is the campaign's "exact rationals by default" claim, and it stays
  owned by the `frac_*` family and checked integer ops.
- `exact_at_scale` — Q operations that are exact *given* the representation
  (Q add/sub, q_lerp on exact inputs); error enters only at input quantization.
- `approximate` — declared-bound cells (`q_sqrt`, future CORDIC trig, `q_sigmoid`).
  These carry `accuracy:` contracts and never advertise exactness.

A model-facing consequence, one line in the extraction prompt when Q lands: *fractions
for exact rational reasoning; Q only when the problem is genuinely real-valued.* GSM8K
stays fraction territory; Q exists for the statistics/geometry/agent-control surface.

---

## Part 2 — the coverage oracle (build before authoring anything)

The mining review proposed, among its candidate packs, at least a dozen cells that
already exist in the library (`digit_sum`, `digit_product`, `digit_reverse`,
`num_digits`, `mode3`, `mean3`, `crt_solve_pair` (as pair), `pow_mod`/`pow_mod_u32`,
`mod_inverse`, `euler_totient`, `factor_count`, `euclid_sq` (= dist_sq),
`point_in_rect`, `fibonacci_checked_u32`, `arithmetic_series_sum`,
`geometric_series_sum`, `choose_u32`, `is_prime_u32`…) and several more that are
compositions of existing cells. That is not a criticism of the review — it is the
strongest possible evidence that **library growth without a coverage map produces
duplicate authoring**, which the admission gate then has to catch one fingerprint at a
time.

**Deliverable: `cell80/data/math_server_catalog_map.json` + `docs/math-server-map.md`.**
Import the math server's function *metadata* (name, domain, category, signature,
exactness), never its implementations, and classify every function into exactly one of:

| status | meaning |
|---|---|
| `covered` | an existing cell (or alias) already does this — name it |
| `composable` | existing cells compose to it — **do not author**; record the composition (planfix's linker makes these free) |
| `candidate` | good cell: exact or boundable, tiny, integer/fraction/Q — enters a wave below |
| `host_only` | math-server escalation territory (floats, symbolic, unbounded) |
| `out_of_scope` | not wanted at either layer |

Candidates sort by: appears in GSM/GSM-Symbolic/agent traces · exact or
`exact_at_scale` · implementation size · verifier value · retrieval distinctness (will
it survive the paraphrase gate next to its siblings?). The map is an **admission
pre-check**: a proposed cell whose catalogue row says `covered` or `composable` is
refused before it ever reaches fingerprinting.

**Typed escalation routing rides on the same map.** The escalation band already names
`needs_floats`/`needs_strings`/`out_of_domain`; the map adds, per `host_only` row, the
math-server route (`domain.function`), so an escalation can surface as
`{code: 0xFF02, cell80_route: null, math_server_route: "trig.sin"}` instead of a bare
code. One static JSON, no runtime machinery — the orchestrator does the routing.

---

## Part 3 — the waves

Ordering rule carried from `library-growth.md`: every wave pays the eval tax (retrieval
rows per cell, paraphrase+adversarial), re-measures the retrieval curve after landing,
and stops if the paraphrase floor degrades. Nothing below blocks the M3 campaign; Wave 1
and Wave Q0 can run in the same worktree sessions that are already touching cells.

### Wave Q0 — Q16.16 plumbing (the prerequisite tier)

The existing Q cells are Q8.8/u16. The useful real-valued range for statistics and
geometry needs Q16.16/u32:

- **Kernel trio as shared inline-foldable kernels** (joining `mul_checked_u32`'s
  family): `q_mul_q16` (word-split partials — the 64-bit-intermediate pattern
  `docs/10` already documents for exactly this kernel), `q_div_q16`, `q_sqrt_q16`
  (integer Newton, `approximate` with declared bound).
- **Scale in the plan unit system** (§1.3) + canonicalization of scale spellings.
- **Accuracy harness in CI** (§1.4) — retrofit declared bounds onto the five existing
  Q8.8 cells as the harness's first users (they currently state limits informally).
- **Byte discipline, measured:** single-site `q_mul_q16` must fold byte-neutral like
  `gcd_u32` did; report the delta the way the checked-kernel factoring did (−1683 B).

### Wave 1 — integer-exact gap fill (no Q needed; from the map, not the taxonomy)

Subject to the coverage map confirming `candidate` status (several will come back
`composable` — that is the map doing its job):

- **Geometry integer subset:** `orientation2d`, `collinear_check`,
  `segments_intersect_int`, `shoelace_area_x2`, `slope_fraction` (returns a `frac` —
  exact), `pythagorean_triple_check`. (`dist_sq`, `manhattan`, `chebyshev`, `dot2`,
  `norm2_sq`, `point_in_rect`, `aabb_intersect` already exist.)
- **Digit/divisibility gaps:** `digital_root`, `palindrome_number_check`,
  `count_trailing_zeros10`. (The `is_divisible_by_k` family is `composable` —
  `divides(k, n)` exists; alias in metadata, don't author.)
- **Combinatorics exact gaps:** `stars_and_bars`, `hypergeometric_count`,
  `multiset_permutations` — checked u32, escalate on overflow. Probability-as-fraction
  is largely `composable` from the `frac_*` family + these counts; record the
  compositions in the map.
- **Sequences:** `collatz_step` / `collatz_steps_bounded` (bounded by construction —
  cycle budget makes the bound honest). `lucas`, `triangular`, `pentagonal` are
  one-expression `composable`/marginal — demand-gate them; they pay retrieval tax for
  near-zero campaign value today.
- **Conversions: not authored — banked.** `dollars_to_cents`, `hours_to_minutes`, etc.
  are owned by the M2.5 unit base-scale table, which applies them deterministically
  inside every plan. A conversion *cell* would duplicate the pass and reintroduce the
  scale-convention bug class the smoke test already paid for. (`cents_mul_qty` and the
  `bps_*` family remain the pattern: money math *at* the canonical scale, not
  conversions *between* scales.)

### Wave 2 — the Q16 packs (after Q0; demand-supported)

- **Statistics:** `zscore_q16`, `stddev_q16` (q_sqrt over the existing exact
  `running_variance_step` output — composition first, cell only if the composition is
  too common to keep re-linking), `percentile_rank_small`, `outlier_iqr_check`,
  `histogram_bucket` (integer). Primary customer is agent control (routing, retry,
  anomaly detection on tool latencies) — the SOMA-adjacent surface — not GSM8K.
- **Geometry Q16:** `midpoint2_q16`, `dist_q16` (q_sqrt ∘ euclid_sq), `lerp2_q16`
  (q_lerp exists at Q8.8; widen).

### Wave 3 — CORDIC trig (demand-gated, explicitly not authored yet)

> **Amended 2026-07-07:** the gate below survives unchanged, but the representation is
> no longer fixed at Q16 — when the counter fires, the customer that fired it picks the
> tier (Q16 or owned f32). See `docs/real-valued-cells-amendment.md` §F2.

`sin_q16`/`cos_q16`/`atan2_q16` via CORDIC (shifts and adds — native to this
substrate's era), `exp_q16`/`log_q16` if ever justified. **Gate:** the `0xFF02` /
`needs_trig`-class escalation counter, instrumented through M3 and the MCP surface's
agent traffic. Fewer than a registered threshold of real fired escalations over the
observation window → the pack stays unauthored and the negative is banked. The
taxonomy's trig column is not demand; a counter is.

### Never (typed escalation, permanent)

Calculus, matrix inverse/eigen (beyond the existing `dot2`/`norm2_sq` vector floor),
real-valued probability distributions, complex arithmetic, symbolic manipulation,
floating constants. All `host_only` in the map with named math-server routes. The
layered stack, stated once: **cells (exact/Q reflex) → math server (broad numeric) →
PAL/Python (arbitrary programs) → frontier.**

---

## Part 4 — registered hypotheses and kill criteria

- **H-Q1 (contracts hold):** every `approximate` cell's measured max error ≤ its
  declared bound across the CI sweep domain, permanently. Not really a hypothesis — a
  CI gate; listed so its failure mode is named: a bound that has to be *loosened* after
  admission is a spec bug and re-runs admission.
- **H-Q2 (kernels fold):** Q16 kernels are byte-neutral at single call sites and net
  positive at ≥2, like `gcd_u32`/`mul_checked_u32`. *Kill:* q_mul_q16 fails to fold →
  the inliner's wide-slot handling has a gap; fix that before growing Wave 2 (the
  kernels are the pack's cost model).
- **H-Q3 (retrieval survives):** paraphrase-split coverage does not drop below the
  post-second-slice floor after each wave. *Kill:* a wave that degrades it pauses
  growth until the index/aliasing work recovers it — the second slice already proved
  this cost is real; this spec inherits the discipline, not the debt.
- **H-Q4 (demand exists for trig):** the escalation counter crosses the registered
  threshold during the observation window. *Kill:* it doesn't → Wave 3 banked
  unauthored, and the "extended libraries" ambition officially ends at Wave 2.

## Honest limits

Q is not exact and this spec never lets it claim to be — the exact story remains
fractions, and any drift of GSM extraction toward Q instead of `frac_*` is a regression
to catch in review, not a feature. The accuracy harness tests declared domains, not all
of `u32` — a bound is evidence over the sweep, like a fingerprint is evidence over the
probe bank. The coverage map is only as good as the trace data feeding its priority
sort, which today is one 20-problem pilot slice and one 123-problem smoke test. And the
demand gate on trig cuts both ways: if the counter stays silent, the honest reading is
that this substrate's callers don't need trigonometry — not that the gate was set wrong.

Source catalogue: [chuk-mcp-math-server](https://github.com/IBM/chuk-mcp-math-server).
