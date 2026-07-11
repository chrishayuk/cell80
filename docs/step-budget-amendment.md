# Step-budget amendment — worst-case IR-step discipline for library cells (v0.2)

**Status:** §3a fixes landed 2026-07-11 (six of seven cells rewritten
value-identically, oracle bill 3.94×10¹² → 5.74×10¹¹ ticks, ~7×); the budget
field + admission enforcement (§2.1–2.2) remain proposed · **Depends on:**
doc 14 Q2 (IR steps as the canonical family cost — operational since the E2
gate meters them bit-exactly on CPU and GPU), the `gate_cost_estimate`
diagnostic (`cell80/tests/msl_battery.rs`) · **Owner:** the library track

**Amendment to the amendment (from doing the work):** §3's `pow_mod` prelude
kernel is *withdrawn*. The exact-behavior constraint decides differently: a
cell's halt/escalation conditions are part of its observable behavior
(`sum_digit_powers` must escalate at exactly the iterative-overflow point),
and the offenders' true costs came from *unbounded trivial iterations*
(`base = 1` looping 65 k times multiplying by one, `day_of_year` adding zero
for months 13…65535), not from missing fast exponentiation. The
value-identical fixes are closed forms and absorbing-state early exits — and
skipping the prelude change means unchanged cells keep their oracle
transcripts. Every rewrite was audited **old-vs-new on the GPU** (300 k
inputs per cell, values + trap status compared, steps deliberately not): the
audit caught one real bug in the first attempt (a `n > 4` guard admitting the
prime 5 into a divisibility shortcut) before any test suite did.

Measured (mean IR steps per random input, 300 k-input GPU audit):

| cell | before | after | fix |
|---|---|---|---|
| `day_of_year` | 2,030,979 | 78 (26,038×) | closed-form cumulative months |
| `pow_small` | 655,238 | 60 (10,920×) | 65535 is absorbing for base ≥ 2; exit on saturate |
| `sum_digit_powers` | 189,353 | 448 (422×) | digits ≤ 1 need no loop; digits ≥ 2 escalate within 32 multiplies |
| `wilson_theorem_check` | 492,004 | 69,093 (7×) | zero-product exit + composite-by-2/3/5 shortcut (Wilson: composite n > 5 ⇒ 0) |
| `wilson_factorial_mod` | 120,298 | 66,403 (1.8×) | k ≥ m answers 0 outright (m divides k!) |
| `is_quadratic_residue` | 655,245 | 361,853 (1.8×) | square symmetry halves the scan; first witness exits |
| `order_modulo` | ~51,000 | unchanged | inherently order-of-a; declared cost (doc already says so) |

The remaining heavies (`is_quadratic_residue`, `order_modulo`,
`wilson_factorial_mod`, prime-input `wilson_theorem_check`) are *honestly*
O(n)-ish by their own doc comments — the declared-budget cases §2 exists for.

**The Z80 finding (why this is a defect class, not a tuning pass).** On the
shipping micro-VM at the default 2M-cycle budget, the old cells didn't run
slow on adversarial inputs — they **refused**: `day_of_year(2024, 65535, 1)`
old = `halt: cycle_budget` at 2,000,015 cycles (never completed) vs new =
`returned 367` in 1,829 cycles; `pow_small(3, 65535)` old = `cycle_budget`
vs new = `returned 65535` in 6,012 cycles. The interpreter and GPU (100M
fuel) accepted inputs the Z80 budget-refused — a cross-target divergence at
real budgets, invisible until the GPU cost-map pointed at exactly these
cells. A worst-case step ceiling at admission (§2.2) would have caught every
one of these before landing.

**Worst-case survey after the fixes** (512-sample GPU probe; the budget
column §2.1 should hold): the whole library fits under ~12k worst-case steps
except six declared-cost cells — `wilson_factorial_mod` (worst 1.71M),
`wilson_theorem_check` (1.35M, prime inputs), `order_modulo` (1.19M),
`is_quadratic_residue` (852k), `triangular_inverse_exact` (271k — newly
visible, the next rewrite candidate), `goldbach_conjecture_check` (148k).
Proposed default ceiling: **2¹⁶ worst-case steps**, grandfathered budgets for
the declared six.

## 1. The finding

The E2/E3 oracle cost-map measured every eligible cell's mean IR-step cost
over the battery's random-u16 input schedule (a worst-case-shaped
distribution, not an operational one). The library's per-cell cost spans
**five orders of magnitude**, and seven cells carry ~99.9% of the whole
10⁶-input gate:

| cell | steps/input (mean, random u16) | share of gate |
|---|---|---|
| `day_of_year` | 1,877,589 | 47.7% |
| `is_quadratic_residue` | 608,335 | 16.1% |
| `pow_small` | 585,990 | 14.9% |
| `wilson_theorem_check` | 442,022 | 11.2% |
| `sum_digit_powers` | 202,563 | 5.1% |
| `wilson_factorial_mod` | 110,732 | 3.2% |
| `order_modulo` | 50,759 | 1.7% |

The next-heaviest cell is ~8k steps; the median library cell is ~10²su.

This is not a battery problem (oracle transcripts already amortize the gate
to seconds). It is a **WCET problem**: the project's claims — microsecond
tools, SIMT-friendly batches (the E2 divergence probe showed the worst warp
lane sets a batch's wall clock), MCU reflex budgets — are worst-case claims,
and a 1.9M-step worst case quietly breaks all three.

## 2. The proposal

1. **Declare it.** A cell's manifest gains a worst-case IR-step budget
   (`step_budget`), the Q2 canonical cost's per-cell contract — T-states and
   GPU wall-time remain per-target refinements of it. Measured, not asserted:
   the GPU steps output (megakernel × random schedule, max not mean) prices
   it for the whole library in one dispatch.
2. **Enforce it at admission.** A candidate whose measured worst case exceeds
   its declared budget (or a library-wide default ceiling, proposed 10⁵
   steps) is refused with a typed reason — exactly the shape of every other
   admission refusal. Existing cells get grandfathered budgets from the
   cost-map and tightened as they're fixed.
3. **Fix the offenders.** Four of the seven are one shared fix: a `pow_mod`
   **fast-exponentiation prelude kernel** (square-and-multiply, O(log n))
   replaces naive O(n) loops in `pow_small`, `is_quadratic_residue` (Euler's
   criterion), `sum_digit_powers`, and `wilson_factorial_mod`'s modpow parts.
   `day_of_year` wants closed-form calendar arithmetic instead of day-by-day
   stepping. `order_modulo` improves with `pow_mod` plus a divisor-of-φ walk.
   **`wilson_theorem_check` is the honest exception:** Wilson's theorem *is*
   an O(p) factorial walk — it keeps a declared (large) budget or a capped
   domain, and its doc comment says so.

## 3. Costs and cautions

- **Prelude changes invalidate every oracle transcript** (the transcript key
  hashes the combined source, prelude included) — adding `pow_mod` means one
  full re-bless (`UPDATE_GOLDEN=1`, ~an hour today, mostly these very cells;
  dramatically less after they're fixed). Sequence the prelude change and the
  cell fixes together so the re-bless is paid once.
- Cell rewrites keep behavior bit-identical (same input→output mapping) —
  family hashes change (source changes), fingerprints don't. The per-pack
  library tests and the admission gate's duplicate check are the safety net.
- `day_of_year` and the day-count pack landed 2026-07-11 (Wave 2) — the fix
  belongs to that pack's owner or coordinates with them; this amendment is
  the request, not the patch.

## 4. Why now

Q2 said IR steps become the canonical family cost; E2 made them measurable
everywhere (interpreter ≡ GPU, per input); the cost-map made the outliers
visible in one launch. A budget nobody can measure is a wish — this one is
now a one-dispatch query, which is exactly when a discipline should become a
gate.
