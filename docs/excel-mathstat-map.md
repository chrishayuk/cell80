# Excel Math&Trig / Statistical functions coverage map

*Status: **landed, 2026-07-11.** Coverage map for Microsoft's Math&Trigonometry (80
functions) and Statistical (111 functions) categories against the 718-cell library, per
`docs/real-valued-cells-spec.md` Part 2's "build before authoring anything" discipline and
`docs/coverage-map-taxonomy-amendment.md`'s taxonomy. Machine-readable form:
[`cell80/data/excel_mathstat_catalog_map.json`](../cell80/data/excel_mathstat_catalog_map.json).*

## What's different about this map

Unlike the Financial (55 functions) and Date&Time (25 functions) maps, this catalogue is
large enough (191 functions) that most of it was classifiable by hand with high confidence
before spending any agent budget: cell80 already has deep `number-theory` (75 cells),
`checked-arithmetic` (41), `statistics`, and `ranking-stats` packs, and the transcendentals/
array-state-field/non-determinism walls are by now well-established, tested precedent (the
original math-server coverage map already found trigonometry and continuous probability
distributions **entirely** `host_only`, `docs/math-server-map.md`). So: **50 genuinely
ambiguous functions were classified by 50 parallel agents** against the live library (the
same rigor as the Financial/DateTime maps); the other **141 were hand-prefiltered** with
reasons recorded per function in the JSON, not left as an unexplained bulk "the rest is
obviously host_only."

## Headline numbers

```
191 functions total
 147 host_only          (77%)  — trig/hyperbolic (21), transcendentals (4 + most of stats' 42),
                                 arbitrary-arity array aggregates (13 math + 55 stats)
  14 covered             (7%)  — already an exact behavioural match
  11 candidate            (6%)  — the real new-authoring backlog
   8 composable-author    (4%)
   7 composable-skip      (4%)
   4 out_of_scope         (2%)  — non-deterministic (RAND family) or a bare constant (PI)
```

## The 14 `covered` — already exact matches, nothing to build

| Excel function | existing cell80 id |
|---|---|
| ABS | `abs_i16` |
| CEILING | `snap_up` / `snap_up_u32` |
| COMBIN | `choose_u32` |
| COMBINA | `choose_with_repetition` |
| FACT | `factorial_checked_u32` |
| FACTDOUBLE | `double_factorial` |
| FLOOR | `snap_down` / `snap_down_u32` |
| INT | `q_to_int_i16` |
| QUOTIENT | `div_floor_u32` / `safe_div` |
| SIGN | `sign_i16` |
| COVARIANCE.S | `sample_covariance_from_sums` |
| SLOPE | `linear_regression_slope` |
| INTERCEPT | `linear_regression_intercept` |
| STANDARDIZE | `zscore_q8` |

This is the strongest confirmation yet that cell80's organically-grown packs already
overlap heavily with Excel's own vocabulary — none of these 14 were built *for* Excel
compatibility, they just already do the same thing under a different name.

## The 7 `composable-skip` — real compositions, not worth a dedicated cell

`LCM`, `SQRTPI`, `CORREL`, `COVARIANCE.P`, `PEARSON`, `RSQ`, `DEVSQ` — each composes
cleanly from 2-3 existing cells (e.g. `RSQ` = `running_correlation_sums_step` →
`correlation` → `q_mul(r,r)`) with no convention fragility or well-known-name pull strong
enough to clear `composable-author`'s bar. Full compositions in the JSON.

## The 19 to build (8 composable-author + 11 candidate)

| function | status | representation | what it needs |
|---|---|---|---|
| CEILING.MATH | composable-author | checked-int | sign/mode case-split over `snap_up_u32`/`snap_down_u32` — the mode-flag negative-number convention is a well-known fragile detail |
| CEILING.PRECISE | composable-author | checked-int | same shape, no mode flag, always rounds toward +∞ regardless of sign |
| EVEN | composable-author | checked-int | `abs_i16` → round-up-to-even → reapply sign |
| FLOOR.MATH | composable-author | checked-int | `div_floor_u32` + `mul_checked_u32` + sign/mode case analysis |
| MOD | composable-author | checked-int (i16) | `div_i16`+`mul_i16`+`sub_i16`, corrected for Excel's divisor-sign remainder convention |
| MROUND | composable-author | checked-int | `round_to_multiple`/`round_to_multiple_u32` + sign handling + the `multiple==0→0` override |
| SQRT | composable-author | f32 | direct `.sqrt()` (fsqrt kernel) — simpler than routing through `nth_root_f32(x,2)` |
| TRUNC | composable-author | Q8.8 (i16) | `div_i16(x, 256)` for the default num_digits=0 case |
| FLOOR.PRECISE | candidate | f32 | `ffloor` kernel + sign-clear + omitted-vs-zero-significance convention |
| ISO.CEILING | candidate | fixed-point Q8.8 signed | ceil-to-multiple generalized to a signed dividend and fractional significance |
| MDETERM | candidate | f32, fixed 3x3 | expose `matrix_solve_3x3`'s internal cofactor-expansion determinant standalone |
| MINVERSE | candidate | f32/exact-fraction, fixed 2x2 (+ optional 3x3) | adjugate construction, one determinant shared across 4 (or 9) outputs |
| ODD | candidate | f32 | round up to nearest odd integer |
| RADIANS | candidate | Q8.8 fixed-point | fixed-pi-constant multiply, matching `geom_circle_area_approx`'s precedent |
| DEGREES | candidate | f32 | single constant multiply (`radians * 180/pi`) |
| ROUND | candidate | f32 | `10^\|digits\|` scale + a new round-half-away-from-zero f32 primitive (doesn't exist yet) + rescale |
| ROUNDDOWN | candidate | checked-int (i16 sign-magnitude) | digit-count-to-power-of-ten scale, truncate toward zero |
| ROUNDUP | candidate | checked-int (i16 sign-magnitude) | same scale, ceiling away from zero |
| STEYX | candidate | statistics-from-sums | `SSE = Syy - Sxy²/Sxx`, divided by `(n-2)`, reusing `correlation`/`std_dev_from_sums`'s bitwise-sqrt technique |

## The 147 `host_only`, three walls, no exceptions found

Same three walls as every prior map, at much larger scale here:

1. **Transcendentals** (25): all 21 trig/inverse/hyperbolic functions (SIN through ACOTH),
   EXP/LN/LOG/LOG10, POWER (fractional exponent). Confirmed empty in
   `rustz80/src/softfloat.rs` — only arithmetic/compare/round/convert kernels exist.
2. **Continuous probability distributions & special functions** (42, all Statistical):
   every `*.DIST`/`*.INV`/`*.TEST` function, GAMMA/GAMMALN/GAUSS/PHI/FISHER/CONFIDENCE*/PROB.
   Matches the original math-server map's own finding — this category has never had an
   exception.
3. **Array-state fields** (68 + 12 non-transcendental math = 80): every function whose real
   Excel signature aggregates an arbitrary-length list — SUM/AVERAGE/MEDIAN/MAX/MIN/COUNT/
   STDEV/VAR/RANK/PERCENTILE/QUARTILE/LARGE/SMALL/FORECAST/TREND/GROWTH/LINEST/LOGEST and
   more. A fixed-arity sibling already existing (`max3`, `median4`, `weighted_sum`, ...)
   does **not** make the real function `covered` — the NPV correction
   (`docs/coverage-map-taxonomy-amendment.md`) established this and it's applied
   consistently here.

Plus 4 `out_of_scope`: `RAND`/`RANDARRAY`/`RANDBETWEEN` (non-deterministic — a direct
conflict with cell80's core guarantee; `stateful-rng`'s seeded PRNGs are the deterministic
alternative) and `PI` (a bare constant, not a computation).

## How this gates authoring

Same rule as every prior map: a row here saying `host_only` or `composable-skip` is refused
before it reaches fingerprinting. The 19 `candidate`/`composable-author` rows above are the
real backlog — re-check each against the *current* `docs/cell-index.md` before authoring,
same caveat as always.
