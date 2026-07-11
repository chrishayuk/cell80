# Math-server coverage map — `chuk_mcp_math` mined against the cell80 library

*Status: **landed, 2026-07-06.** The Part 2 deliverable from
`docs/real-valued-cells-spec.md` ("the coverage oracle — build before authoring
anything"). Machine-readable form: [`cell80/data/math_server_catalog_map.json`](../cell80/data/math_server_catalog_map.json).*

*Update (2026-07-07): the first slice of the 77 candidates has been authored — the
number-theory family (`mobius_function`, `little_omega`, `big_omega`,
`divisor_power_sum`, `jordan_totient`, `carmichael_lambda`), see
`docs/library-growth-log.md`'s "Wave 6" note. This snapshot's `candidate` rows for those
six are now stale (they're `covered`); the JSON itself is left as-authored evidence
of the mining pass rather than hand-edited — re-check against the live
`docs/cell-index.md` before authoring anything from the remaining ~71 candidates, per
this doc's own "How this gates authoring" section below.*

*Update (2026-07-07, Wave 7): the figurate-numbers slice landed as four cells —
`polygonal_number`, `is_polygonal_number`, `centered_polygonal_number`,
`square_pyramidal_number` — see `docs/library-growth-log.md`'s "Wave 7" note. This
snapshot's `pentagonal_number`/`is_pentagonal_number`/`star_number` candidate rows
are now stale for a subtler reason than an exact-name match: they're `covered` by
**generalization**, not by a same-named cell — `polygonal_number(5, n)` is
`pentagonal_number`, `is_polygonal_number(5, x)` is `is_pentagonal_number`, and
`centered_polygonal_number(12, n)` is `star_number(n+1)` (1-indexed). The pack
READMEs' auto-generated "not yet built" lists (`gen_pack_readmes.py`, exact-name
matching only) still show these three — a known blind spot of that matcher, not a
gap in the library; verify against `docs/cell-index.md` before treating any
generated "not yet built" entry as ground truth.*

*Update (2026-07-07, Wave 8): the recursive-sequences slice landed as two cells —
`lucas_u_v`, `tribonacci_number` — see `docs/library-growth-log.md`'s "Wave 8" note.
Same generalization-blind-spot pattern as Wave 7: `pell_number` and
`pell_lucas_number` are now `covered` by `lucas_u_v(2, 1, n)`'s U and V outputs
respectively, but the pack README generator's exact-name matcher still lists both
as "not yet built" — check `docs/cell-index.md`, not the generated README, before
treating either as an open candidate. Also landed the same day: six digit-operation
cells (`digital_root`, `persistent_digital_root`, `is_palindromic_number`,
`next_palindrome`, `is_repdigit`, `is_automorphic_number`) from this map's
`digital_operations` category — those all matched by exact name, no stale rows.*

*Update (2026-07-07, Wave 9): the modular/classic-number-theory slice landed as five
cells — `extended_gcd`, `jacobi_symbol`, `order_modulo`, `is_quadratic_residue`,
`discrete_log_naive` — see `docs/library-growth-log.md`'s "Wave 9" note. All five matched
by exact name; no generalization-blind-spot rows to flag this time.*

*Update (2026-07-08, Wave 10): the combinatorial-numbers slice landed as four cells —
`bell_number`, `stirling_first`, `stirling_second`, `is_catalan_number` — see
`docs/library-growth-log.md`'s "Wave 10" note. All four matched by exact name. First wave
to use a local array in the dialect (verified standalone before designing around it).*

*Update (2026-07-08, Wave 11): the geometry/vector integer subset's first slice landed
as three cells — `geom_distance_3d`, `vectors_parallel`, `cross_product` — see
`docs/library-growth-log.md`'s "Wave 11" note. All three matched by exact name.
`triple_scalar_product`/`triple_vector_product` (this map's remaining vector
candidates) were deliberately deferred to a follow-up wave for complexity/risk
reasons, not built and not forgotten.*

*Update (2026-07-08, Wave 12): `triple_scalar_product` and `triple_vector_product`
landed — see `docs/library-growth-log.md`'s "Wave 12" note. Both matched by exact name.
This closes out every `linear_algebra.vectors` candidate this map named.*

*Update (2026-07-08, Wave 13): `matrix_det_2x2`, `matrix_solve_2x2`, `covariance`,
`linear_regression_slope` landed — see `docs/library-growth-log.md`'s "Wave 13" note. All
four matched by exact name. `correlation`/`effect_size_r` (this map's remaining
`statistics.descriptive`/`statistics.inference` candidates, both Q8.8) deferred to a
follow-up — the last two candidates from this map's original 77.*

*Update (2026-07-09, Wave 14): `correlation` and `effect_size_r` landed — see
`docs/library-growth-log.md`'s "Wave 14" note. Both matched by exact name, both Q8.8
fixed-point via the same scale-before-sqrt precision technique `q_sqrt` itself
uses. This closes out every `candidate`-classified function this map named —
**the original 77-candidate list is now fully resolved** (landed, folded into a
generalization, or explicitly deferred with a documented reason).*

## What this is

`chuk-mcp-math-server` (the MCP server named in the spec) depends on a separate
package, `chuk_mcp_math` (PyPI `chuk-mcp-math==0.2.3`), which is where its 642
advertised functions actually live. Rather than importing or executing any of that
package, its **sdist tarball was downloaded and every `.py` file parsed with Python's
`ast` module** — a static read, never a `pip install` or `import` of third-party code.
Each `@mcp_function`-decorated function's name, namespace, category, description,
signature, and docstring were extracted this way; `642` unique function names came
out (`653` raw decorator sites, `11` of them the package's own internal duplicates —
the same function re-exported under an identical name from a second file, e.g.
`advanced_operations.py`'s grab-bag module re-exporting things also defined in their
proper category file).

Every one of the 642 functions was then classified against the current **259-cell**
cell80 library (`cell80/cells/*.rs`, via `cargo run -q -p cell80 --bin cell80 --
index cell80/cells --json`) into exactly one status:

| status | meaning |
|---|---|
| `covered` | an existing cell80 cell already does this — cites the cell80 `id` |
| `composable` | 1-2 existing cell80 cells already compose to this — cites the composition |
| `candidate` | genuinely new, bounded, and a good fit — the real output of this exercise |
| `host_only` | needs floats/transcendentals/closures/unbounded output — not a cell80 fit |
| `out_of_scope` | low value, or a duplicate of another entry in this same catalogue |

## Headline numbers

```
642 functions total
  420 host_only     (65%)
   77 candidate     (12%)  <- the real Wave 1/2 backlog
   58 covered       ( 9%)
   51 composable    ( 8%)
   36 out_of_scope  ( 6%)
```

By namespace (the catalogue's own top-level grouping):

| namespace | total | host_only | candidate | covered | composable | out_of_scope |
|---|--:|--:|--:|--:|--:|--:|
| arithmetic (number theory + core) | 420 | 258 | 57 | 49 | 24 | 32 |
| trigonometry | 71 | 71 | 0 | 0 | 0 | 0 |
| statistics | 35 | 19 | 4 | 3 | 7 | 2 |
| numerical | 25 | 20 | 1 | 2 | 1 | 1 |
| linear_algebra.vectors | 23 | 6 | 4 | 1 | 12 | 0 |
| timeseries | 20 | 15 | 4 | 1 | 0 | 0 |
| geometry | 12 | 4 | 5 | 2 | 1 | 0 |
| linear_algebra.matrices | 10 | 7 | 2 | 0 | 0 | 1 |
| probability | 10 | 10 | 0 | 0 | 0 | 0 |
| calculus | 9 | 9 | 0 | 0 | 0 | 0 |
| conversions | 7 | 1 | 0 | 0 | 6 | 0 |

The `host_only` mass is dominated by three permanent, structural reasons, not just
"needs a float":
1. **No closures.** The dialect can't accept a function argument at all, so anything
   in `calculus` (derivative/integral/root-of-a-function) or `numerical.optimization`
   (gradient descent, Nelder-Mead, ...) is impossible to express, not merely
   float-dependent.
2. **Unbounded output.** Anything returning a variable-length list (all primes up to
   n, a whole time series, a Kaprekar sequence) doesn't fit fixed 16-bit registers.
3. **Genuinely irrational/continuous math.** Trigonometry, probability distributions,
   series approximations of transcendentals — the dialect's permanent non-goals.

`trigonometry` (71) and `probability` (10) and `calculus` (9) are **entirely**
`host_only` — confirming the spec's own Wave 3 gate (trig) and Never-list (calculus,
continuous probability) rather than finding an exception to either.

## The 77 candidates (the real backlog)

73 are exact-integer/fraction; 4 are Q-format (2 × Q8.8, 2 × Q16.16 — none of these
should be built before Wave Q0 lands). Full reasoning for every one is in the JSON;
grouped highlights by area:

**Number-theoretic arithmetic functions** (a coherent, well-motivated family —
`euler_totient`/`sum_divisors`/`factor_count` already exist but none of their
siblings do): `mobius_function`, `little_omega`/`big_omega`, `divisor_power_sum`
(generalizes both `factor_count` and `sum_divisors` with an exponent, the same
"missing general-parameter sibling" shape as `weighted_sum2`/`weighted_sum`),
`jordan_totient` (generalizes `euler_totient`), `liouville_function`,
`carmichael_lambda`.

**Figurate numbers** (cell80 only has the triangular special case):
`pentagonal_number`/`is_pentagonal_number`, `polygonal_number`/`is_polygonal_number`
(general s-gonal), `centered_polygonal_number`, `star_number`,
`square_pyramidal_number`.

**Recursive sequences beyond Fibonacci**: `pell_number`, `pell_lucas_number`,
`tribonacci_number`, `lucas_u_v` (generalized Lucas sequences).

**Digit operations beyond one-shot `digit_sum`**: `digital_root` (exact closed form,
`1+(n-1) mod 9`), `persistent_digital_root` (iteration count — subsumes the
catalogue's own duplicate `digital_persistence`, see below), `is_palindromic_number`,
`next_palindrome`, `is_repdigit`, `is_automorphic_number`, `digit_sort`,
`number_to_base`.

**Modular arithmetic / classic number theory**: `extended_gcd` (standalone Bézout
coefficients — `mod_inverse`/`crt_solve_pair` only inline this internally today),
`jacobi_symbol`, `order_modulo`, `is_quadratic_residue`, `discrete_log_naive`
(bounded by a caller-supplied max exponent), `solve_linear_diophantine`,
`wilson_theorem_check`/`wilson_factorial_mod`.

**Combinatorics**: `bell_number`, `stirling_first`/`stirling_second`,
`is_catalan_number` (the inverse-membership test — distinct from the already-shipped
`catalan_number`).

**Geometry** (the spec's own Wave 1 list, confirmed against the real catalogue):
`geom_segment_intersection` matches the spec-named `segments_intersect_int` gap
exactly; `geom_distance_3d` is the missing 3D sibling of `euclid_sq`;
`geom_line_intersection` extends it to an exact-fraction intersection point;
`geom_circle_area`/`geom_circle_circumference` are Q16.16 (fixed-π).

**Vectors (3D, exact)**: `cross_product`, `triple_scalar_product`,
`triple_vector_product` (BAC-CAB identity — reduces to pure dot-products and scalar
multiplies), `vectors_parallel` (maps onto the spec's own already-named but unbuilt
`orientation2d` gap).

**A genuine matrix exception**: `matrix_det_2x2` (`ad-bc`, 4 args, exact) — small
enough that the project's own "vector floor" exception to the matrix non-goal
plausibly extends to it; `matrix_solve_2x2` (Cramer's rule, two fractions sharing a
denominator) is the same idea at higher complexity, lower priority.

**Statistics, given precomputed sums** (not raw arbitrary-length lists — that
aggregation stays upstream): `covariance` (exact, mirrors `running_variance_step`'s
bivariate case), `linear_regression`'s slope (exact fraction), `correlation` (Q8.8,
needs `q_sqrt`/`q_div`), `effect_size_r` (Q8.8, two scalar inputs).

**Deferred behind the still-open array-state-cell question** (flagged, not built):
`simple_moving_average`, `weighted_moving_average`, `rolling_variance`, `rolling_std`
— all need a sliding window (remember the last N values), which no cell80 cell has
ever done. Distinct from the already-shipped `running_variance_step` (cumulative
over the whole stream, not windowed).

**One arithmetic-cell precision finding worth flagging on its own**:
`series_sum(first, last, count)` — the same arithmetic-series sum as the already-shipped
`arithmetic_series_sum(a, d, n)`, just parameterized by endpoints instead of
(start, step). The obvious composition — `avg2(first, last) * count` — is
**unsound**: `avg2` floors before the multiply, so an odd `first+last` silently
corrupts the result. Confirmed directly by the classifying agent rather than
assumed safe. A dedicated cell (or deriving `d` then calling `arithmetic_series_sum`,
which costs an extra exact-division step) is the correct fix — recorded here so
nobody "obviously" composes it wrong later.

## Cross-chunk duplicates caught in merge review (a limitation worth stating plainly)

The 642 functions were classified by 4 parallel agents, each seeing only its own
slice of the catalogue — genuinely necessary to keep any single prompt a manageable
size, but it means an agent can't catch a duplicate that appears in a *different*
slice. Two real ones surfaced only when the results were merged and reviewed by hand:

- **`extended_gcd`** (arithmetic/number_theory) and **`bezout_identity`**
  (arithmetic/bezout_identity) are the exact same computation (`ax+by=gcd(a,b)`,
  returning the same `x,y` coefficients) under two names, defined in two different
  source files. Kept `extended_gcd` as the candidate; `bezout_identity` reclassified
  to `out_of_scope` pointing at it.
- **`persistent_digital_root`** (arithmetic/digital_operations) and
  **`digital_persistence`** (arithmetic/special_numbers) both compute "steps to
  reach a single digit via repeated digit-summing." Kept `persistent_digital_root`
  (its `(n, base)` signature is all-integer; `digital_persistence`'s `(n,
  operation: str)` needs string handling the dialect doesn't have anyway).

Three `covered` entries were also caught misclassified during a spot-check pass
(citing a composition rather than a single cell80 id) and moved to `composable`:
`mean`, `range_value` (both: fixed-arity already exists as `mean3`/`range3`; the
arbitrary-*n* streaming case composes from `accumulate_step`+`safe_div` or
`running_min_max_step`), and `is_divisible` (an exact match to `divides`, just with
swapped argument order).

**The honest reading**: this map is evidence gathered under real constraints (4
independent slices, no shared context), not a proof of zero remaining duplication —
the same epistemic status the admission gate's own fingerprint agreement claims for
itself. Treat a `candidate` entry as "not yet found to be covered," not as
guaranteed-novel; the admission gate itself is the final, authoritative check at
authoring time.

## How this gates authoring

Per the spec: a proposed cell whose catalogue row says `covered` or `composable` is
refused before it ever reaches fingerprinting. Concretely, before authoring any
cell from the 77 `candidate` rows above, re-check it against the *current* (not
this snapshot's) `docs/cell-index.md` — the library keeps growing, and a candidate
recorded here in July 2026 may be covered by something landed since.

Typed escalation routing (the spec's other Part 2 item — surfacing a
`math_server_route` alongside an escalation code) is **not** implemented by this
map; it would need per-`host_only`-row route strings (e.g. `trigonometry.sin`), which
the current JSON doesn't carry. Left for whoever picks up that specific piece.
