# Step-budget amendment — worst-case IR-step discipline for library cells (draft v0.1)

**Status:** proposal · **Depends on:** doc 14 Q2 (IR steps as the canonical
family cost — operational since the E2 gate meters them bit-exactly on CPU and
GPU), the `gate_cost_estimate` diagnostic (`cell80/tests/msl_battery.rs`) ·
**Owner:** the library track (coordinates with whoever owns the affected packs)

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
