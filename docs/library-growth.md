# Growing the cell library — toward a large prebuilt collection

> **Before authoring a new cell, check [`docs/cell-index.md`](cell-index.md)** — every
> landed cell, grouped by pack, generated from the actual library (not hand-transcribed, so
> it can't drift the way this file's old cell-count prose once did). It's the fastest way to
> confirm a behaviour doesn't already exist before writing a duplicate the admission gate
> would just refuse.

*The goal is a **big, growing library of prebuilt cells** — hundreds of small, distinct,
deterministic integer utilities an agent can retrieve, run, and compose, organized into
**packs**. cell80's whole pitch is "millions of tiny tools, retrieved" — so the library should
be **broad**: the more genuinely-distinct behaviours sit on the shelf, the more an agent finds
one instead of writing code. This guide is how to grow it well.*

## Contents

This file is the **living guide** — the stable rules for growing the library. The
chronological wave-by-wave growth history (everything from Phase 2.3 onward) was split into
[`library-growth-log.md`](library-growth-log.md) on 2026-07-11; new dated wave entries go
there, append-only. Line numbers, not anchors (the file is edited concurrently enough by
parallel sessions that hand-verified line numbers are more trustworthy than guessed GFM
anchors):

- The shape we're building toward — line 46
- What a good cell is — line 562
- Two rules that keep a *large* library strong — line 571
- Principles (what makes a cell worth adding) — line 593
- The contribution rule (every new-cell PR) — line 626
- Packs (organise discovery by family via tags) — line 642
  (ends with the currently-open gaps — Q16.16, array-state-field)
- The growth history (moved) — pointer to `library-growth-log.md`
- After authoring: re-run the evals — line 806

**For the current cell count and per-pack list, don't trust any number in this file's own
prose (including this TOC's line numbers drifting as the file grows) — check
[`docs/cell-index.md`](cell-index.md), generated fresh from the live library.**

## The shape we're building toward

The wave table below is the maintained at-a-glance ledger (every new wave appends a row).
Where a row says "see the pack note below": notes through the 163-cell checkpoint are in
"Landed pack notes" further down this file; later waves' notes are in the dated entries of
[`library-growth-log.md`](library-growth-log.md).

```
wave 1 ✓   59 cells   predicates · safe arithmetic · bounds · percent · ranking · bit/mask
wave 2 ✓   98 cells   + number theory · distance · bit/encoding · hashing · stats · conversion
wave 2.5    96 cells   + the wide u32-in-state siblings (square_wide, weighted_sum_wide);
                        4 folded into aliases by the admission gate (see below)
wave 3a   100 cells   + calendrical/checksum, first slice (is_leap_year, days_in_month,
                        day_of_week, luhn_check) — ISBN/IBAN/UPC deferred (see below)
wave 3b   103 cells   + Q8.8 fixed-point, first slice (q_mul, q_div, q_lerp) —
                        q_sqrt/piecewise sigmoid-tanh still open
wave 3c   108 cells   + agentic runtime primitives, first slice (token_bucket_step,
                        backoff_next, circuit_breaker_step, debounce_step, hysteresis) —
                        rate_window_update still open
wave 3d   111 cells   + running statistics, first slice (running_min_max_step,
                        streak_step, accumulate_step) — Welford variance still open
wave 3e   114 cells   + spatial/grid, first slice (grid_index, point_in_rect,
                        aabb_intersect) — Morton encode/decode, Bresenham still open
pilot     120 cells   + the Phase 2.3 pilot batch: packing/BCD (pack_u8, pack_nibbles,
                        bcd_encode, bcd_decode) + vector (dot2, norm2_sq) —
                        unpack_lo/unpack_hi never built (exact duplicates of
                        low_byte/high_byte, caught before authoring)
wave 3f   128 cells   + the GSM8K math campaign, M1 pack 1/5: checked/exact arithmetic
                        (mul_u16_u16_to_u32, add_checked_u32, sub_checked_u32,
                        div_exact_u32, div_floor_u32, div_ceil_u32, mod_u32, fits_u16) —
                        see `docs/math-campaign-spec.md`
wave 3g   134 cells   + M1 pack 2/5: money/basis-points (bps_of, increase_by_bps,
                        decrease_by_bps, original_before_bps_increase,
                        original_before_bps_decrease, cents_mul_qty) — tax/tip/markup
                        consolidated into one increase_by_bps cell rather than three
                        near-identical ones
wave 3h   138 cells   + M1 pack 3/5: units (same_unit_check, unit_mul, unit_div,
                        unit_cancel_check) — the campaign's first free-fn pack (all
                        prior GSM8K cells were u32 state cells)
wave 3i   142 cells   + M1 pack 4/5: verifier/ranker (sum_equals, diff_equals,
                        product_equals_u32, quotient_equals_exact_u32) — reverse-
                        equation-satisfaction checks that always return a verdict,
                        never escalate; answer_eq/multi-plan-agreement/tie-break
                        needed no new code (see the pack note below)
wave 3j   145 cells   + stateful/RNG, first slice (lcg_next, xorshift16,
                        counter_step) — bounded_rand deferred (exact duplicate of
                        safe_mod)
wave 3k   149 cells   + signed deltas, first slice (sign_i16, abs_i16, clamp_i16,
                        apply_delta_clamped) — the library's first i16 cells; also
                        widened DEFAULT_PROBES (fingerprint.rs) with a negative-i16
                        value, which fixed the long-standing snap_down false
                        positive as a side effect (see the pack note below)
wave 3l   153 cells   + scoring/choice, second slice (weighted_sum2, weighted_sum3,
                        choose_best3, is_clear_winner) — generalizes weighted_sum's
                        fixed weights to caller-supplied ones (a genuine u32
                        overflow is now reachable and escalates); choose_best3
                        picks by score where value != score, unlike argmax3
wave 3m   163 cells   + the GSM8K math campaign, M1 pack 5/5 (final): fractions
                        (frac_reduce/add/sub/mul/div/cmp/eq, is_integer,
                        frac_to_mixed, ratio_split2) — M0 (u32-across-a-call-
                        boundary) landed as Tier 2 (one u32 param per call), so
                        each cell inlines its own GCD-reduction loop rather than
                        sharing a two-u32-param gcd_u32 helper; M1 complete
wave 3n   203 cells   + the GSM8K math campaign, M1 second slice: closing the gap
                        against the spec's original ~95-cell estimate —
                        checked-arithmetic +18 (mul/mul3/mul_add/mul_sub/pow
                        checked, wide siblings of min/max/clamp/range_check/
                        avg2/abs_diff/divides/gcd/lcm, and the sign-magnitude
                        kernels docs/math-campaign-spec.md names as an M0
                        prerequisite), fractions +9 (reciprocal, of_whole vs
                        scale, min/max, ratio_split3, is_proper, the mixed-number
                        pair), money-bps +2 (bps_increase/decrease_between, the
                        missing rate-from-before/after inverse), verifier-ranker
                        +11 (wide siblings plus reverse-equation counterparts for
                        every new checked-arithmetic shape), and a genuine gap
                        found in units (a wage-rate dimension, money/time, was
                        unmodeled — extended same_unit_check/unit_mul/unit_div/
                        unit_cancel_check's dispatch tables rather than adding
                        near-duplicate cells). See the pack note below for what
                        was deliberately *not* built (money-bps and units'
                        raw-count gaps against the spec turned out to already be
                        covered, mirroring the score_2factor/bounded_rand
                        precedent) and the retrieval-curve cost this batch paid.
wave 3o   208 cells   + third slice, small and deliberately so: completes the
                        sign-magnitude algebra (smag_mul/smag_div, alongside
                        smag_add/sub/cmp), two more fraction shapes (frac_avg2,
                        frac_sub_from_whole — the subtract-direction sibling of
                        frac_add_whole), and lcm3 (number-theory's gcd/gcd3
                        pairing extended to lcm — inlines the shared-kernel
                        prelude's gcd twice, since lcm itself isn't in
                        CELL_PRELUDE). A smaller batch than the second slice, on
                        purpose: the five-way smag_add/sub/cmp/mul/div family
                        shares near-identical vocabulary and structural shape, a
                        same-shape sibling confusion no wording fix resolves
                        (confirmed empirically — see the pack note below).
wave 3p   209 cells   + fourth slice, two cells closing the last small gaps
                        identified in the five campaign packs: smag_eq (the
                        sign-magnitude family's missing verifier — completes
                        the pattern every other checked op got an `_equals`
                        counterpart) and a ninth unit-dimension code,
                        rate_count_per_time (count/time — a production-rate
                        word problem, "N per hour", previously unmodeled,
                        alongside the wage-rate fix). Retrieval cost negligible
                        (smag_eq's own direct query hits rank 1). Beyond these
                        two, further math-cell growth is paused in favor of the
                        campaign's own intended mechanism: M2/M3 precipitation
                        via `cell_solve` (in progress, `feat/cell-solve`) — real
                        problems surface which schemas actually recur, rather
                        than more speculative hand-authoring.
wave 3q   221 cells   + MATH/AIME pack, first slice — an explicit, one-time
                        override of the pause above (requested ahead of M3's
                        read-out, not a reversal of it): wide modular
                        arithmetic (pow_mod_u32, mod_add_u32, mod_sub_u32,
                        mod_mul_u32), number-theory scalars (sum_divisors,
                        euler_totient, smallest_prime_factor, digit_reverse,
                        digit_product), and a new combinatorics pack
                        (factorial_checked_u32, choose_u32, permute_u32) —
                        see the pack note below and
                        docs/math-campaign-spec.md's "scoped ahead of the
                        gate" section.
wave 3r   225 cells   + MATH/AIME pack, second slice — the four items the
                        first slice deferred: is_prime_u32 (wide sibling of
                        is_prime; cost scales with sqrt(n), documented rather
                        than silently slow), shoelace_area_x2 (a new
                        geometry pack; the signed-arithmetic chain the first
                        slice judged not worth the cost, built this time),
                        mod_inverse (extended Euclid, the Bezout coefficient
                        tracked as a sign-magnitude pair), and crt_solve_pair
                        (two-congruence CRT, inlining mod_inverse's own
                        algorithm since it can't be called as a subroutine).
                        Closes out every candidate docs/math-campaign-spec.md
                        originally scoped except count_divisors/dist_sq
                        (still exact duplicates of factor_count/euclid_sq —
                        never built, not deferred).
wave 3s   232 cells   + the "straightforward deferred set" backlog: q_sqrt,
                        q_sigmoid (fixed-point), running_variance_step
                        (running-stats), morton_encode/morton_decode,
                        bresenham_step (spatial/grid), rate_window_update
                        (agentic-runtime) — see the pack note below. q_tanh
                        was scoped but not built: it reduces exactly to
                        clamp_i16(x, -256, 256), now tagged on that cell
                        instead of shipped as a second one.
wave 3t   239 cells   + a broad geometry/combinatorics/sequences batch,
                        requested directly rather than pulled from a
                        standing backlog: shoelace_area_x2_quad,
                        triangle_is_valid (geometry); fibonacci_checked_u32,
                        catalan_number, derangement_count (combinatorics);
                        arithmetic_series_sum, geometric_series_sum
                        (a new sequences pack) — see the pack note below.
                        sort3 (returning (min, mid, max) as one call) was
                        scoped but refused by the admission gate: a real,
                        structural finding, not a false positive — see the
                        pack note.
wave 4a   244 cells   + wave 4, slice 1/5 — width/precision gap-fill,
                        redirected from a dead PlanFix role/op/slot-validator
                        proposal to the two concrete gaps PlanFix's own
                        findings actually named: is_lt_u32/is_gt_u32/
                        is_le_u32/is_ge_u32 (the missing wide-predicate
                        family — only answer_eq_u32 existed at u32 width)
                        and frac_of_whole_floor (the floor sibling of
                        frac_of_whole, which only had the exact-or-escalate
                        variant) — see the pack note below.
wave 4b   249 cells   + wave 4, slice 2/5 — scoring/choice generalization:
                        argmax3_u32/argmin3_u32/clear_winner_u32 (wide
                        siblings past the u16 ceiling) and choose_best2/
                        choose_worst2 (the 2-candidate siblings of
                        choose_best3, absorbing the original proposal's
                        choose_lowest_cost2/choose_highest_profit2 as tags
                        rather than shipping four near-identical cells) —
                        see the pack note below.
wave 4c   253 cells   + wave 4, slice 3/5 — sequences nth-term gap-fill:
                        arithmetic_nth_u32/geometric_nth_checked_u32 (the
                        missing single-term siblings of
                        arithmetic_series_sum/geometric_series_sum, which
                        only ever summed the whole sequence),
                        triangular_inverse_exact (the missing inverse of
                        triangular), and consecutive_sum_start (one
                        step-parameterized cell replacing the original
                        proposal's separate odd/even "consecutive sum"
                        variants) — see the pack note below.
wave 4d   256 cells   + wave 4, slice 4/5 — verifier-ranker gap-fill:
                        percent_equals_bps (money-bps's first verifier
                        sibling — every other checked-arithmetic shape
                        already had one), parts_sum_to_total4_u32 (the
                        missing four-way sibling of sum3_equals_u32), and
                        nonnegative_after_delta (a boolean-verdict form of
                        apply_delta_clamped's sign-handling idiom) — see
                        the pack note below.
wave 4e   259 cells   + wave 4, slice 5/5 (final) — agentic-runtime
                        reflexes: cooldown_step (plain decrement-to-zero,
                        distinct from counter_step/backoff_next),
                        epsilon_greedy_pick3 (explore/exploit selection),
                        zscore_q8 (Q8.8 z-score given an already-computed
                        stddev). retry_budget_step/budget_spend_step
                        confirmed behaviourally identical to
                        token_bucket_step(refill=0) and folded into its
                        tags; ucb1_score_q8 not attempted (needs a
                        fixed-point ln with no dialect primitive, same
                        class as cosine_score_approx) — see the pack note
                        below. Wave 4 complete: 239 -> 259 cells (~20 net
                        new, down from the ~100 originally proposed).
wave 5a   261 cells   + M2.6 slice gap-fill: sum4 (the four-operand sibling
                        of sum3) and scale_percent_u32 (the percent-of core
                        the widened arithmetic lane resolves to) — landed
                        alongside the M2.5/M2.6 canonicalization pass, not a
                        standalone wave at the time (see
                        docs/math-campaign-amendment.md).
wave 5b   263 cells   + AIME geometry pair, from the post-M2.9 gap analysis's
                        "cheap, high-yield" recommendation: cos_frac_from_sides
                        (law of cosines rearranged to an exact sign-magnitude
                        fraction — no square root, no trig) and heron_16a2
                        (16*Area^2 via the four-factor form, always integer
                        for a valid triangle). Both convert a slice of AIME
                        geometry from "needs a real number" to "fraction/integer
                        arithmetic the dialect already has." Paired with the
                        mod-space rewrite (a canon.rs compiler feature, not a
                        cell — see docs/math-campaign-amendment.md's
                        "mod-space rewrite" status note).
wave 6    269 cells   + math-server number-theory family, six cells drawn
                        directly from docs/math-server-map.md's 77-candidate
                        coverage map (docs/real-valued-cells-spec.md Wave 1,
                        the "ready-now, no Q16.16 prerequisite" slice):
                        little_omega/big_omega (distinct vs.
                        multiplicity-counted prime factors), mobius_function
                        (the classic sign/squarefree function, the library's
                        first i16-returning free fn since the signed-deltas
                        pack), divisor_power_sum (sigma_k, generalizing
                        factor_count/sum_divisors with an exponent —
                        weighted_sum2's "missing general-parameter sibling"
                        shape again), jordan_totient (generalizing
                        euler_totient with an exponent k), and
                        carmichael_lambda (the reduced totient, lcm-combined
                        over prime-power components). See the pack note below.
wave 7    273 cells   + figurate numbers, the math-server map's next slice
                        (docs/math-server-map.md's figurate_numbers category):
                        polygonal_number generalizes the s-gonal formula
                        (s=3 reproduces triangular, s=4 is the perfect
                        squares, s=5 is pentagonal, s=6 is hexagonal — one
                        cell instead of a differently-named one per side
                        count, folding the map's own separately-listed
                        pentagonal_number candidate into it before writing
                        a line of code), is_polygonal_number is its
                        membership predicate (folding is_pentagonal_number
                        the same way), centered_polygonal_number folds
                        star_number in as its s=12 case one ring later
                        (star_number(k) = centered_polygonal_number(12,
                        k-1)), and square_pyramidal_number is the checked-
                        u32 sum-of-squares sequence, landed as its own
                        cell since 1+4+9+...+n^2 isn't reducible to the
                        s-gonal formula. See the pack note below.
wave 8    281 cells   + recursive sequences + digit operations, the
                        math-server map's next two slices. lucas_u_v(p, q,
                        n) generalizes the two-term Lucas recurrence
                        (U(n)=p*U(n-1)+q*U(n-2), V(n)=p*V(n-1)+q*V(n-2))
                        for non-negative p, q — folding the map's
                        separately-listed pell_number (U at p=2,q=1) and
                        pell_lucas_number (V at p=2,q=1) into it before
                        writing a line of code, the same move wave 7 made
                        for pentagonal_number; tribonacci_number is its own
                        cell since a 3-term recurrence isn't reducible to
                        lucas_u_v's 2-term family. Plus six digit-operation
                        cells: digital_root (closed form), the additive-
                        persistence step-count sibling
                        persistent_digital_root, is_palindromic_number
                        (any base, via digit-reversal comparison),
                        next_palindrome (bounded upward search, escalates
                        past the u16 ceiling), is_repdigit, and
                        is_automorphic_number (n^2 ends with n). See the
                        pack note below.
wave 9    286 cells   + modular / classic number theory, the math-server
                        map's next slice: extended_gcd (the standalone
                        two-Bezout-chain extended Euclidean algorithm --
                        mod_inverse/crt_solve_pair each only inline one
                        chain internally today), jacobi_symbol (i16-typed,
                        via the standard reciprocity reduction tracked as
                        a parity flip rather than a signed accumulator),
                        order_modulo (multiplicative order, bounded by n),
                        is_quadratic_residue (any modulus, direct search),
                        and discrete_log_naive (brute-force search bounded
                        by a caller-supplied max exponent). See the pack
                        note below.
wave 10   292 cells   + combinatorial numbers, the math-server map's next
                        slice (286 -> 288 in between via the F-wave
                        session's own softfloat pack landing in the same
                        checkout, unrelated to this track): bell_number
                        and stirling_first both needed a small local
                        array (the first library cells to use one) --
                        verified the syntax compiles standalone before
                        committing to either design. stirling_second uses
                        the inclusion-exclusion closed form instead (sign-
                        magnitude accumulation, no array), and
                        is_catalan_number walks catalan_number's own
                        recurrence inline as a bounded membership search.
                        See the pack note below.
wave 11   295 cells   + 3D vector basics, the geometry/vector integer
                        subset's first slice: geom_distance_3d (euclid_sq's
                        missing 3D sibling, an excess-32768 coordinate
                        shift avoids ever forming a signed i16 subtraction
                        that could overflow i16's own range), vectors_parallel
                        (cross-product component equality via paired signed
                        products, no combining step needed), and
                        cross_product (full sign-magnitude tracking through
                        both the multiply and the combining subtract,
                        checked against a 2,000-case random sweep).
                        triple_scalar_product/triple_vector_product
                        deliberately deferred — see the pack note below.
wave 12   297 cells   + the vector pack's deferred triple products,
                        completing the geometry/vector integer subset:
                        triple_scalar_product (a . (b x c), reuses
                        cross_product's own computation as its first
                        stage, then a signed dot) and
                        triple_vector_product (a x (b x c) via the
                        BAC-CAB identity, never an actual cross product).
                        Both cross-checked against a 2,000-case random
                        sweep each. See the pack note below.
wave 13   301 cells   + matrix (matrix_det_2x2, matrix_solve_2x2 — the
                        "vector floor" exception to the matrix non-goal
                        extended exactly this far, per
                        docs/math-server-map.md's own scoping) and
                        statistics from precomputed sums, not raw
                        datasets (covariance, linear_regression_slope —
                        both exact signed fractions over a shared
                        positive denominator, the same "two fractions
                        sharing a denominator" shape matrix_solve_2x2
                        uses). correlation/effect_size_r (Q8.8, needing
                        q_sqrt/q_div) deferred — see the pack note below.
wave 14   303 cells   + Q8.8 statistics gap-fill: correlation,
                        effect_size_r (deferred from wave 13) — closes
                        out the original 77-candidate math-server map
                        in full. See the pack note below.
next      ~4           + Wave Q0 (Q16.16 plumbing) as a prerequisite for
                        the 4 Q-format candidates it unlocks;
                        cosine_score_approx (deferred until cell_solve
                        reads out); CORDIC trig remains demand-gated per
                        docs/real-valued-cells-spec.md Wave 3 — none of
                        this remains from the original 77-candidate
                        math-server map
```

All five originally-planned wave-3 packs (calendrical/checksum, fixed-point, agentic
runtime, running statistics, spatial/grid) landed a first slice; each deferred its harder
items (see the per-pack notes below and `docs/cell-index.md`'s "planned" section). The
Phase 2.3 pilot batch (packing/BCD + vector) exercised the author→verify→admit loop
end-to-end for the first time — see the "Phase 2.3" section below for what it found.

**The GSM8K math campaign (`docs/math-campaign-spec.md`), M1 pack 1/5: checked/exact
arithmetic.** `mul_u16_u16_to_u32`, `add_checked_u32`, `sub_checked_u32`, `div_exact_u32`,
`div_floor_u32`, `div_ceil_u32`, `mod_u32`, `fits_u16` — all `u32` state cells (a free-fn
entry still can't take/return `u32`, the same constraint the calendrical/checksum pack
found). "Checked" here means the escalation contract (`halt(0xFF05)`, `needs_wider_math`,
Phase 3.2), not the saturating/sentinel convention `add_sat`/`safe_div` already use — a
caller can tell "this genuinely didn't fit" apart from an ordinary result, which a
saturated or sentinel value can't. `div_ceil_u32` computes via `a/b` plus a remainder
check rather than `(a+b-1)/b`, so a large `a` can't overflow the *overflow guard itself* —
"checked" was the whole point. Confirmed by testing directly: **M0's u32-across-a-
call-boundary prerequisite is still unbuilt**, even after `Cond32` (u32 comparisons)
landed from a parallel session mid-pack — a local helper function can't yet take or
return `u32`, so the fraction pack (which wants a shared `gcd_u32` reducer) stays blocked
until that lands.

**M1 pack 2/5: money/basis-points.** `bps_of`, `increase_by_bps`, `decrease_by_bps`,
`original_before_bps_increase`, `original_before_bps_decrease`, `cents_mul_qty` — basis
points (1% = 100 bps), never float percentages, so `value * bps / 10000` stays exact
integer math. All six escalate (`halt(0xFF05)`) on multiply overflow via the same
"divide back and compare" trick as the checked-arithmetic pack
(`let p = a.wrapping_mul(b); if a != 0 && p / a != b { halt(...) }`) rather than needing a
u64. Checked `docs/cell-index.md` before authoring and dropped several near-duplicates of
the checked-arithmetic pack from the original spec's money list (`cents_add`/`cents_sub`
duplicate `add_checked_u32`/`sub_checked_u32`; `cents_div_qty`/`unit_price_cents`
duplicate `div_floor_u32`; `price_total`/`change_due` duplicate `cents_mul_qty`/
`sub_checked_u32`), and consolidated `tax_bps`/`tip_bps`/`markup_bps` — identical formula
under different names — into one canonical `increase_by_bps` cell instead of shipping
three copies. `cents_mul_qty` is kept distinct from `mul_u16_u16_to_u32` even though both
multiply: `mul_u16_u16_to_u32` takes two `u16`s and always fits `u32` exactly, while
`cents_mul_qty`'s `unit_cents` is already a wide `u32` and can genuinely overflow — a
real behavioural difference, not a renamed duplicate. "Cents" names the minor unit of any
decimal currency (cents, pence, kopecks, ...), not USD specifically — kept over a more
generic `minor_units` name because it's the de facto term in decimal-money code regardless
of actual currency (Stripe et al. use `amount_in_cents` the same way).

**M1 pack 3/5: units.** `same_unit_check`, `unit_mul`, `unit_div`, `unit_cancel_check` —
the campaign's first *free-fn* pack (every prior GSM8K cell was a `u32` state cell; these
are plain arity-2 `u16` functions, so they go through the admission gate's fingerprint
check for real rather than being exempted as a state cell — confirmed no collisions).
Dimension codes: `0=count, 1=money, 2=time, 3=distance, 4=area, 5=volume,
6=rate_money_per_count, 7=rate_distance_per_time` — a fixed small enum with hand-written
pairwise composition rules (`count*money=money`, `distance*distance=area`,
`money/count=rate_money_per_count`, same-unit-divided-by-itself always cancels to
`count`, ...), not a general symbolic exponent-vector algebra; unmodeled pairs escalate
(`halt(0xFF06)`, `out_of_domain` — genuinely a *different* escalation reason than the
arithmetic packs' `needs_wider_math`, since a unit mismatch isn't a wide-math problem).
`same_unit_check` returns the shared dimension code on a match (useful for tagging an
addition's result) rather than a bare boolean, so it doubles as the compatibility check
for both `+` and `-` (same requirement) instead of shipping a second, redundant
`unit_add_check`. `unit_cancel_check` is `unit_div`'s table restated as a non-escalating
predicate, for a caller (e.g. a future plan verifier) that wants to try several candidate
unit pairs without committing to a halt. Landed 4 of the spec's `~10` estimate — the
smaller set is what's concretely load-bearing for the one worked example in
`docs/math-campaign-spec.md` (`count * rate_money_per_count = money`, round-tripped
through `unit_div(money, count)` first); a full exponent-vector unit algebra is deferred
until real GSM8K plans demand it.

**M1 pack 4/5: verifier/ranker.** `sum_equals`, `diff_equals`, `product_equals_u32`,
`quotient_equals_exact_u32` — each re-derives one side of a candidate plan's claimed
equation and returns a plain `0`/`1` verdict, **never escalating**: a verifier's whole job
is to answer, so a genuine overflow (`product_equals_u32`) or a divide-by-zero
(`quotient_equals_exact_u32`) is just a `0` (the claim doesn't hold), not a
`halt(0xFF05)` hand-off — a deliberately different contract from the arithmetic packs,
which compute a value and escalate when they can't. `sum_equals` widens its addition to a
local `u32` internally (no function boundary, so M0 doesn't block it) so a genuine `u16`
overflow can't wrap into a false match. Checking `docs/cell-index.md` and the spec's own
`~20`-cell estimate before authoring found most of it already covered by existing cells,
so nothing new was built for: `answer_eq` (an exact alias of the predicates pack's `eq` —
no new code), multi-plan agreement and tie-breaks (`majority3`/`mode3`, ranking-stats,
already do "do at least two of three plans agree" and "which value repeats"), and range
constraints (`range_check`, validation pack). `answer_in_options` (checking an answer
against an arbitrary-length option list) is deferred — GSM8K is free-response, not
multiple-choice, so the motivation is thin, and a real implementation would need an array
state field this session hasn't risked yet.

**Stateful/RNG, first slice.** `lcg_next` (seed = seed \* 1664525 + 1013904223 mod 2^32,
Numerical Recipes constants, top 16 bits returned), `xorshift16` (x ^= x<<7; x ^= x>>9;
x ^= x<<8 — a distinct recurrence from lcg_next, no multiply), `counter_step` (a modular
increment-and-wrap counter for round-robin dispatch). `bounded_rand` — the fourth item on
`library-growth.md`'s own next-waves list — was **not** built: `raw % bound` (0 on
`bound == 0`) is an exact behavioural duplicate of the already-shipped `safe_mod`, the
same reasoning that folded `wrap` into `safe_mod` as an alias earlier. Verifying these
cells surfaced an important, easy-to-miss point about `StateCell`/`Runner::run`'s
contract: **a state cell does not persist memory across separate `.run()` calls** —
`Runner::run`'s own doc says "memory the previous run touched is zeroed first, so
repeated runs start from the same clean state." A naive test that calls `.run()` twice on
the same instance expecting `self.seed`'s prior mutation to carry over silently resets to
0 — not a compiler bug (confirmed by reproducing the identical pattern against the
already-shipped `streak_step`, whose own host-oracle test already re-`set`s the carried
field from the previous `.get()` before every call, per `cell80/tests/library.rs`). Every
"step" cell in the library (this pack included) relies on the *caller* threading the
carried field through explicitly, matching the real host/agent loop's own calling
convention (`run_state`/`run_state_fast` take the full current field set on every call) —
documented here since it cost real debugging time to pin down and will bite the next
stateful pack's author again if it isn't written down.

**Signed deltas, first slice.** `sign_i16` (-1/0/1), `abs_i16` (magnitude as `u16`,
correctly handling `i16::MIN`'s 32768 which doesn't fit back in `i16`), `clamp_i16` (the
signed counterpart of `clamp`), `apply_delta_clamped` (apply a signed delta to an unsigned
value, clamped to `[0, cap]` — a health/resource/score adjustment that can't go negative
or over a cap; the "risk delta" use case). The library's first `i16` cells, now that the
dialect supports it (confirmed directly: `i16` params, returns, comparisons, unary
negation, and `as`-casts between `i16`/`u16` all work). `lerp_i16` (interpolating between
two signed values) is deferred — signed multiply/divide's rounding direction and overflow
safety haven't been worked out.

Authoring `sign_i16` surfaced a second real fingerprint-probe gap, the same class as the
`luhn_check`/`is_zero` case from Wave 3: every value in `DEFAULT_PROBES` is non-negative
when reinterpreted as `i16` (the largest, `1230`, is still far short of the `i16` sign
bit), so `sign_i16`'s negative branch never fired on the bank alone — it degenerated to
`nonzero` (agreement 1.00, both only ever emitting `0`/`1`). Fixed the same honest way:
widened `DEFAULT_PROBES` (`cell80/src/fingerprint.rs`) with `[65531, 3]` (`-5` as an
`i16` bit pattern), rather than touching `sign_i16`. That widening had a welcome side
effect: it also separated the long-documented `snap_down`/`round_to_multiple` false
positive (they'd agreed on the whole ten-probe bank since Wave 3 but diverge at e.g.
`x=8, step=5`) — the twelfth probe happens to be one of the inputs where they disagree
(`snap_down(65531, 3) = 65529` vs `round_to_multiple(65531, 3) = 65532`), so the gate is
now fully clean: 149 admitted, 0 refused, not the long-standing 1.

**Scoring/choice, second slice.** `weighted_sum2`/`weighted_sum3` generalize
`weighted_sum`/`weighted_sum_wide`'s fixed weights (1, 2, 3) to caller-supplied ones —
`score_2factor` from the original next-waves list is the same formula as `weighted_sum2`
under a different name, so its vocabulary was folded into `weighted_sum2`'s tags rather
than shipping a duplicate. Unlike their fixed-small-weight siblings (whose additions can
never overflow `u32`), arbitrary weights genuinely can, so both escalate
(`halt(0xFF05)`) on a real `u32` overflow instead of silently wrapping — verified directly
(`a=b=wa=wb=65535` overflows; `a=1000,wa=1000,b=1,wb=1` doesn't overflow `u32` but does
saturate the `u16` return, exactly like `weighted_sum_wide`'s existing convention).
`choose_best3` picks the *value* of whichever of three (value, score) pairs has the
highest score — genuinely different from `argmax3`, which only works when the value you
want back **is** the value being compared; ties go to the lowest index, matching
`argmax3`'s own convention. `is_clear_winner` checks whether a margin is decisive (top −
second ≥ margin) rather than just picking a winner, catching the "basically a tie" case
`argmax3`/`choose_best3` can't express. `tie_break_*` from the original list was too
under-specified to build — every existing ranking cell already bakes in its own concrete
tie-break rule (lowest index for `argmax3`/`choose_best3`, "value that repeats" for
`mode3`), so a separate abstract tie-break cell has no clear GSM8K/agent-facing use case
yet.

**Fractions — GSM8K math campaign, M1 pack 5/5 (final).** `frac_reduce`, `frac_add`,
`frac_sub`, `frac_mul`, `frac_div`, `frac_cmp`, `frac_eq`, `is_integer`, `frac_to_mixed`,
`ratio_split2` — every fraction is a `(u32, u32)` numerator/denominator pair; every op
reduces the result to lowest terms via an inline Euclidean GCD. `frac_cmp`/`frac_eq`
cross-multiply (`na*db` vs `nb*da`) instead of reducing first, so they work correctly on
unreduced-but-equivalent fractions (`1/2` vs `2/4`) without extra steps. `frac_sub`
escalates (`halt(0xFF05)`, `needs_wider_math`) if the result would be negative — an
unsigned fraction can't represent it, the same convention `sub_checked_u32` already
uses for "this would go negative." Every op escalates (`halt(0xFF06)`, `out_of_domain`)
on a zero denominator, and `frac_div` additionally on a zero-numerator divisor
(dividing by a zero fraction). `frac_floor`/`frac_ceil` from the spec's own list were
never built — checking `docs/cell-index.md` before authoring found they'd be exact
duplicates of the already-shipped `div_floor_u32`/`div_ceil_u32`.

M0 (u32-across-a-call-boundary) landed from a parallel session mid-session, as **Tier
2**: at most one `u32` parameter per call (and it must be the first), a `u32` return, and
nothing more — confirmed directly, and confirmed this does *not* fully unblock the
originally-envisioned design. A shared `gcd_u32(a: u32, b: u32) -> u32` reducer (what the
whole session assumed "M0 landing" meant) still can't be called: **two** `u32` params in
one call still isn't supported — `docs/10-dialect-semantics.md` calls this out itself as
"the honest residual: a two-wide-param kernel... still doesn't fit three registers —
widen inside, or pass through state." The actual unblock is narrower and was available
all along: a `while` loop over `u32` **local variables**, entirely inside one cell's own
`run` method, was never gated by the call-boundary limitation at all (only actual
function *calls* passing `u32` arguments are) — confirmed directly with a standalone
inline-GCD test before writing any real cell. So every fraction cell duplicates its own
short Euclidean-GCD loop rather than sharing one; a genuine `gcd_u32` helper (for the
next pack that wants one) still needs a further compiler feature, not this one.

Cells are also **modular** now: a shared kernel prelude (`gcd`, `imin`, `imax`, `iabs_diff`,
`isqrt`, `clamp_to`) is appended to every cell and dead-code-eliminated, so `lcm` calls `gcd`
and `chebyshev` calls `iabs_diff`/`imax` instead of re-implementing them — and a cell that uses
no kernel stays byte-identical to having no prelude.

Big is the point — but **big and distinct**, not big and padded. Every cell earns its place by
being a *different behaviour*, not a renamed one.

## What a good cell is

> A tiny, deterministic utility an agent needs often but shouldn't spend tokens re-deriving.

```
small · deterministic · easy to test · easy to describe · useful in many workflows
cheap enough to run constantly · a distinct behaviour · part of a confusable family
```

## Two rules that keep a *large* library strong

A library that just accumulates functions rots in two ways — these rules prevent it:

1. **No behavioural duplicates — aliases live in metadata, not in code.** `time_until(now,
   deadline)` is `sub_sat`; `deadline_missed` is `is_ge`; inclusive `between` is `range_check`.
   Don't ship a second cell with the same behaviour — add the alias as a **tag/summary** on the
   existing one so search still finds it. Duplicates *hurt* retrieval (two right answers = no
   signal) and bloat the shelf without adding capability. **Enforced, not just requested:** the
   Phase 2.2 admission gate (`cell80 index --gate`, `cell80/src/admission.rs`) found four
   cells that had shipped as exact duplicates without anyone noticing — `argmin2` ≡ `is_gt`,
   `argmax2` ≡ `is_lt`, `quantize` ≡ `safe_div`, `wrap` ≡ `safe_mod`, the identical formula
   under a different name for every `u16` input. All four were removed and their vocabulary
   merged into the surviving cell's tags.
2. **Grow in confusable families, and pay the eval tax per cell.** Retrieval only gets *teeth*
   from 3-4+ cells per family that collide in text but differ in behaviour; composition needs
   predicates + transforms that chain. A new cell ships with its eval pressure or it's just
   inventory. See the contribution rule.

So "a large number of cells" and "good evals" pull the *same* direction: more distinct
confusable cells = a bigger shelf *and* a harder, more honest retrieval benchmark.

## Principles (what makes a cell worth adding)

- **Fits the integer envelope** (`u8`/`u16`/`u32`/`i16`, no float/string/syscall, bounded
  cycles). The compile error *is* the "this belongs in host code" signal. `i16` has landed
  (signed compare/divide/`>>`), so signed deltas are in-envelope; the unsigned abs-via-a-branch
  idiom (`abs_diff`/`manhattan`) remains fine where a cell doesn't need negatives.
- **≤ 3 args, or a state cell.** The calling convention takes 3 args (`HL`/`DE`/`BC`); a cell
  that needs more (4-point distance, multi-weight scoring) is a **state cell** (a `struct` with
  named fields + `fn run(&mut self)`), like `manhattan`/`chebyshev`.
- **Small and pure** — tens of bytes of behaviour, deterministic, cycle-honest.
- **Composes** — produces/consumes values others use; include **boolean predicates** (`-> 0/1`).

### What the compiler gives you (author cells clean)

- **Comparisons are values:** `fn run(a: u16, b: u16) -> u16 { (a < b) as u16 }`. All six.
- **`&&` / `||`** (short-circuit): `((lo < x) && (x < hi)) as u16` — and a short-circuit guard
  is the right way to keep a loop in-budget, e.g. `while r < 255 && (r+1)*(r+1) <= n { … }`.
- **Runtime bit shifts:** `x << bit` / `x >> bit` with a *variable* amount (a shift ≥ 16
  saturates a `u16` to `0`) — bit/rotate/encoding cells are one-liners.
- **Multi-function cells:** a cell may define helpers and call them; the entry is `run`
  (e.g. `lcm` calling a local `gcd`).

### Standardise these semantics

- **Predicate convention:** `false = 0`, `true = 1` (built on `bool as u16`).
- **Divide/remainder by zero:** unguarded `/` and `%` **halt the cell**
  (`Halt::DivByZero`, the Phase 0.3 default) rather than returning a value — **guard
  explicitly** when zero is a valid input (`if b != 0 { a / b } else { 0 }`); `safe_div`/
  `safe_mod` are canonical.
- **`u16` overflow is silent** (wraps); saturating cells (`add_sat`, `mul_sat`, `sum3`, …) cap
  at `65535`; percent/scale/`euclid_sq`/`triangular` assume their product fits `u16` (beyond is
  the host-code signal).

## The contribution rule (every new-cell PR)

```
1. cell80/cells/<pack>/<name>.rs                — header (//! summary, //! tags:) + fn/struct
2. cell-eval/datasets/retrieval.jsonl           — a direct row that ranks the cell #1
                                                   (verify with `cell80 search`), + paraphrase
3. composition or adoption task (if user-facing) — composition_tasks.jsonl / tasks.jsonl
4. cell80/tests/library/<pack>.rs                — edge-case rows (the host oracle)
5. docs/cell-index.md                            — regenerate (command at the top of the file)
```

Steps 1-2 are enforced, not just requested: `cell80 index cell80/cells --gate
cell-eval/datasets/retrieval.jsonl` (the Phase 2.2 admission gate, `cell80/src/admission.rs`)
refuses a candidate that's behaviourally identical to an already-shipped cell (alias it in
metadata instead) or that carries no retrieval rows to survive. Run it before opening a PR.

## Packs (organise discovery by family via tags)

**Update (2026-07-07):** a pack is now a real directory, not just a tag — cells live in
`cell80/cells/<pack>/<id>.rs` (`cell80/scripts/gen_cell_index.py` infers each cell's pack
from its parent directory via `cell80 index --json`'s `"pack"` field, so there's no
hand-maintained pack list to keep in sync anymore), and the test suite mirrors the same
layout (`cell80/tests/library/<pack>.rs`, `docs/library-growth.md`'s own contribution rule
above). This reverses the original design below on direct request, once the library's
growth made a flat 269-file directory and a single 3,300-line test file hard to navigate.
Build packs out broadly:

```
math-core      bounds        percent       ranking-stats   number-theory   distance
bitops         bit-encoding  hashing       packing         time            budget
validation     vector        decimal       random/stateful scoring/choice  conversion
```

### Landed pack notes (through the 163-cell checkpoint; the wave table above is current)

See **[`docs/cell-index.md`](cell-index.md)** for the full, generated, per-pack list — not
duplicated here, so there's exactly one place this can go stale (and it's checked against
the real library every time it's regenerated).

**Calendrical / checksum, first slice (wave 3): `is_leap_year`, `days_in_month`,
`day_of_week`, `luhn_check`.** A real constraint surfaced authoring this pack: a free-fn
cell's calling convention is 16-bit registers, so **`u32` can only exist as a state field,
not a call param or return type** — a classic multi-digit checksum (Luhn over a real
13-19-digit card number, ISBN-13, IBAN mod-97) needs far more digits than even a `u32` state
field holds. `luhn_check` is scoped to a `u16` input (≤ 5 decimal digits, documented via
`//! limits:`) to stay a plain free function; ISBN-10/13, IBAN mod-97, and UPC are deferred
until either a state-cell version (carrying digits as array/state fields) or wider host-side
preprocessing is worth the design cost.

**Q8.8 fixed-point, first slice (wave 3): `q_mul`, `q_div`, `q_lerp`.** Sidesteps the same
`u32`-in-a-free-fn constraint by keeping params/return as `u16` and widening only as a
*local* (`a as u32 * b as u32`, `>> 8u32`) — the pattern any Q8.8 free function should
follow. `q_lerp` also serves as an EMA step (`q_lerp(prev, sample, alpha)`) — deliberately
*not* shipped as a second `q_ema` cell, since the formula is identical; the admission gate
would refuse it anyway. `q_sqrt` and `q_sigmoid` landed later (see the "straightforward
deferred set" pack note above) — `q_tanh` didn't: it reduces exactly to `clamp_i16(x, -256,
256)`, tagged on that cell instead of shipped as a second one.

**Agentic runtime primitives, first slice (wave 3): `token_bucket_step`, `backoff_next`,
`circuit_breaker_step`, `debounce_step`, `hysteresis`.** All genuinely need state (each
depends on outcomes from prior calls, not just this call's arguments), unlike the other
"time/budget" names already flagged in Next waves below — `used_percent`/`fits_budget`/
`cooldown_remaining` turned out to be aliases of `percent`/`is_le`/`sub_sat` respectively
and were never built, exactly the kind of check `docs/cell-index.md` is for. `backoff_next`
guards against a real overflow: doubling `current` directly can wrap past `u16::MAX` before
the cap check runs, so it compares against `cap / 2` first and only multiplies when doubling
is provably safe. `rate_window_update` landed later (see the "straightforward deferred set"
pack note above) — the simpler "N events per fixed window" shape, distinct from
`token_bucket_step`'s smooth refill-and-spend model.

**Running statistics, first slice (wave 3): `running_min_max_step`, `streak_step`,
`accumulate_step`.** Deliberately doesn't reach for Welford's algorithm (which needs care in
fixed point) or a histogram (which needs array state fields, not yet exercised by any landed
cell) — instead `accumulate_step` keeps a running sum + count and composes with the
already-landed `safe_div` for the mean, rather than shipping a monolithic "running mean"
cell that would just re-implement `safe_div` internally. A fixed-point running variance
landed later as `running_variance_step` (see the "straightforward deferred set" pack note
above) — it turned out not to need the compounding-truncation care Welford's algorithm is
usually reached for: recomputing the mean fresh from the exact running sum on each side of
the update, rather than carrying a previously-truncated running mean forward, sidesteps the
concern this note originally deferred on. Percentile-from-histogram is still open, gated on
the array-state-field question this variance cell didn't need to answer — **now confirmed, not
just suspected** (`experiments/sliding-window-state-cells-findings.md`): every `Runner::run()`
zeros the previous run's writes before applying this run's inputs, so a state cell's only
"memory" is whatever the host re-supplies as named scalar inputs each call, and the named-field
surface (`StateCell`, `CellHost::run_state`) round-trips scalars only, never array fields. A
hand-authored `simple_moving_average` (an 8-sample ring buffer) verified its own window/head/
sum logic and the compiler's array-field layout correct end to end via a raw-address round
trip, then failed silently past the first call through every real driving surface — exactly
the class of wrong-answer-with-no-error the admission gate exists to keep out of the shipped
library, so it stays unlanded in `experiments/` until the round-trip surface exists. That
surface is the same primitive Phase S3's `bytes[N]`/`str[N]` byte-buffer I/O already needs and
never built (`docs/09-cell80-abi.md`) — one design should cover both element widths, not two
separate builds; the open questions (element width, whole-envelope vs. logical-length, per-field
vs. whole-state-blob round-trip) are written up in the findings doc. `weighted_moving_average`/
`rolling_variance`/`rolling_std` (`docs/math-server-map.md`'s mining) are blocked on the
identical gate and should land as a batch right behind `simple_moving_average` once it does.

**Spatial / grid, first slice (wave 3): `grid_index`, `point_in_rect`, `aabb_intersect`.**
`grid_index` is a plain arity-3 free function; the other two are state cells purely for arg
count (6 and 8 named fields respectively), not width. Both containment checks are
half-open — edge-touching does not count as inside/overlapping, verified by hand for both.
Morton encode/decode were deliberately not attempted this slice: encoding a full `u16` x/y
pair needs a 32-bit interleaved result, so — like the calendrical/checksum pack's
discovery — it would need a `u32` state field, and the bit-interleaving loop itself
(computed shift amounts on a wide accumulator) hasn't been risked yet. Both landed later
(see the "straightforward deferred set" pack note above) using the classic branch-free
"magic numbers" bit-spread — constant shift amounts throughout, so the dynamic-shift
question this note raised never actually came up. A Bresenham line stepper also landed
later, redesigned around a real constraint found along the way: state fields can't be
`i16` at all, so the stepper tracks only `dx`/`dy`/the error term (the last as a
sign-magnitude pair) and reports 0/1 step flags rather than signed coordinates directly.

**Packing/BCD + vector, the Phase 2.3 pilot batch: `pack_u8`, `pack_nibbles`, `bcd_encode`,
`bcd_decode`, `dot2`, `norm2_sq`.** The first real run of the author→verify→admit loop
(below) — and it immediately earned its keep: checking `docs/cell-index.md` before
authoring found `unpack_lo`/`unpack_hi` would be exact duplicates of the already-landed
`low_byte`/`high_byte`, so they were never written at all (a real duplicate caught before
the gate ever had to refuse one). `cosine_score_approx` was scoped out for a different
reason — an honest one: exact fixed-point cosine similarity needs `sqrt(norm_a * norm_b)`
without overflowing a `u16` return, and that hasn't been worked out yet. `dot2` is a state
cell purely for arg count (4 fields: two 2D vectors), not width.

### Next waves (prioritized — keep them distinct)

- **scoring / choice — second slice landed** (see the pack note above): `weighted_sum2`,
  `weighted_sum3`, `choose_best3`, `is_clear_winner`. `score_2factor` folded into
  `weighted_sum2`'s tags (identical formula). **`choose_best4`/`weighted_sum4` landed in the
  90-cell workflow batch** (2026-07-11, "Systematic family expansion" above) — the
  straightforward 4-candidate generalization, built once the pattern was clearly repeated.
  `tie_break_*` still open, still under-specified — no concrete use case beyond what
  `argmax3`/`choose_best3`/`mode3` already bake in.
- **vector — `cosine_score_approx` landed (2026-07-11).** The long-open "overflow-safe
  sqrt-of-a-product" blocker, closed once `isqrt_u32` existed (the 90-cell batch's own
  addition): two u16-bounded norms always fit a u32 product with room to spare
  (`65535*65535 < u32::MAX`), so the missing piece was the wide sqrt kernel, not a design
  problem. Built by hand, not delegated, and verified against hand-computed cases before
  landing (`cell80/cells/vector/cosine_score_approx.rs`).
- **stateful / RNG — first slice landed** (see the pack note above): `lcg_next`,
  `xorshift16`, `counter_step`. `bounded_rand` was checked against `docs/cell-index.md`
  and found to be an exact duplicate of `safe_mod` — not built. (`ema_update`/
  `moving_avg_update` — skip, `q_lerp` already is this: `q_lerp(prev, sample, alpha)` is
  one EMA step.) **`xorshift32`/`counter_step_u32`/`pingpong_step` landed in the 90-cell
  batch** — wide/new siblings of the same family.
- **time / budget** — checked against `docs/cell-index.md` before building: `used_percent` is
  `percent`, `fits_budget` is `is_le`, `cooldown_remaining` is `sub_sat`, `time_until` is
  `sub_sat`, `deadline_missed` is `is_ge` — all aliases, none of these get built as new cells.
- **signed deltas — first slice landed** (see the pack note above): `sign_i16`, `abs_i16`,
  `clamp_i16`, `apply_delta_clamped`; the 90-cell batch added `negate_i16`/`min_i16`/
  `max_i16`/`abs_diff_i16`. **`lerp_i16` landed (2026-07-11)** — the "signed multiply/divide
  rounding direction and overflow safety not yet worked out" blocker, closed with the
  sign-magnitude pattern `linear_solve_1var`/`linear_eq_holds` proved out: `b-a` can exceed
  `i16`'s own representable range even when `a`/`b` are both valid `i16` (e.g. `i16::MAX` to
  `i16::MIN`), so it's computed via `(magnitude, sign)` throughout, never a native `i16`
  subtract. Shipped as a 3-arg free function (matching `q_lerp`'s own convention), not a
  state cell — an early draft over-defaulted to one out of habit, caught and simplified
  before landing. Built by hand and verified against hand-computed cases, including the
  `i16::MAX`/`i16::MIN` boundary, before landing (`cell80/cells/signed-deltas/lerp_i16.rs`).
- **Q16.16 fixed-point plumbing — checked, still genuinely blocked.** Not a design gap this
  session could close: a Q16.16 `q_mul` needs a 64-bit intermediate the dialect doesn't
  have (`docs/10-dialect-semantics.md`). Real compiler-level work, already on the
  multi-target track's own roadmap (WS-C's Q16.16 stretch, `docs/13-multi-target-spec.md`)
  — not something to force from the library side.
- **array-state-field gap — CLOSED (2026-07-11, `.cell` v11).** `u16[N]`/`u32[N]` state
  fields round-trip by name (`StateCell::set_array`/`get_array`,
  `CellHost::run_state_values`; the scalar `run_state` lanes refuse array-state cells loudly
  instead of running them with an unfed window), wire code 6 in `state_addrs`, admission
  shape-classes arrays by element type + length, and the fingerprint drives elements
  cyclically. The whole sliding-window family landed with it (`simple_moving_average` —
  promoted verbatim from the experiment — plus `weighted_moving_average`,
  `rolling_variance`, `rolling_std`; 740 admitted, 0 refused). Design close-out in
  `experiments/sliding-window-state-cells-findings.md`. Still open nearby:
  percentile-from-histogram (now expressible, not yet authored) and Phase S3's
  `bytes[N]`/`str[N]` byte-I/O, which rides the same wire mechanism when it comes.

## The growth history (moved)

Everything from Phase 2.3 onward — the wave-by-wave growth records (Phase 2.3's
author→verify→admit loop, ecosystem mining, the 90-cell workflow batch and its rounds,
checkpoint 21, Finance80 Waves 1–2), with their gate results and banked negatives — lives in
[`library-growth-log.md`](library-growth-log.md), append-only and dated. New wave entries go
**there**; this file keeps only the standing rules and the currently-open gaps.

## After authoring: re-run the evals

Each new **family** is a retrieval test case; each **predicate + transform** pair a composition
test case. Re-run `cell-eval retrieval` / `composition`. Expect direct P@1 to stay strong while
paraphrase stays in coin-flip territory as the library grows (8 → 98 cells: direct ≈ 0.92,
paraphrase 0.53 → 0.45) — that gap is the standing case for the **type-led / capability index** (rank by
typed signature + a `kind = predicate | transform | …` first, embeddings as the tiebreaker). A
big confusable library is precisely what makes that benchmark trustworthy.
