# Cell index — every landed cell, by pack

*Generated from `cell80/cells` (306 cells) by `cell80/scripts/gen_cell_index.py`. Regenerate after any cell is added/removed:*

```
cargo run -q -p cell80 --bin cell80 -- index cell80/cells --json \
  | python3 cell80/scripts/gen_cell_index.py > docs/cell-index.md
```

See `docs/library-growth.md` for the packs' purpose, the contribution rule, and the admission gate that enforces "no behavioural duplicates."

## agentic-runtime (8)

| id | signature | summary |
|---|---|---|
| `backoff_next` | `Backoff::run() -> u16` | Capped exponential backoff: next = min(current * 2, cap), starting at 1 when current is 0. |
| `circuit_breaker_step` | `CircuitBreaker::run() -> u16` | Circuit-breaker state machine step: closed(0) counts failures and opens at the threshold; open(1) waits for cooldown then tries half-open(2); half-open resolves to closed on success or back to open on failure. |
| `cooldown_step` | `CooldownStep::run() -> u16` | Countdown-to-ready state cell: decrements cooldown by 1 (floored at 0) each call, reporting 1 once it reaches 0 — distinct from counter_step (modular increment, round-robin) and backoff_next (exponential growth); no existing agentic-runtime cell does a plain decrement-to-zero. |
| `debounce_step` | `Debounce::run() -> u16` | Debounce a noisy 0/1 signal: only confirms a change to `input` once it's held for `threshold` consecutive steps; output is the last confirmed-stable value. |
| `epsilon_greedy_pick3` | `EpsilonGreedyPick3::run() -> u16` | Epsilon-greedy selection: returns alt_idx (explore) if rand_bps < epsilon_bps, else best_idx (exploit) — composes with the already-shipped lcg_next/xorshift16 (for rand_bps, via safe_mod against 10000) and epsilon_bps as a basis-points exploration rate (e.g. 1000 = 10% exploration). |
| `hysteresis` | `Hysteresis::run() -> u16` | Hysteresis (Schmitt-trigger) state: turns on at value >= high, turns off at value <= low, else holds the prior state (the dead zone between them). |
| `rate_window_update` | `RateWindowUpdate::run() -> u16` | Fixed-window rate limiter step: given the current time `now`, the running window's start and size, and the count so far, rolls over to a fresh window (starting at `now`) once `now - window_start >= window_size`, then allows the event if `count < limit` (incrementing count) — distinct from token_bucket_step's smooth refill-and-spend model, this is the simpler "N events per window" shape. The caller threads window_start/count through repeated calls, matching backoff_next/token_bucket_step's convention. |
| `token_bucket_step` | `TokenBucket::run() -> u16` | Token-bucket rate limiter step: refill by `refill`, cap at `capacity`, then try to spend `cost`; 1 if allowed, 0 if not enough tokens (tokens still refill either way). Also a plain retry/spend budget when called with refill=0 and capacity >= tokens: retry_budget_step and budget_spend_step are the same formula under different names, confirmed directly (cell80/tests/library.rs) rather than shipped as separate cells. |

## bit-encoding (9)

| id | signature | summary |
|---|---|---|
| `bit_length` | `run(x: u16) -> u16` | Number of bits needed to represent x: index of the highest set bit + 1 (0 for x == 0). |
| `high_byte` | `run(x: u16) -> u16` | High byte of x (x >> 8). |
| `leading_zeros` | `run(x: u16) -> u16` | Count of leading (high) zero bits in the 16-bit value (16 for x == 0). |
| `low_byte` | `run(x: u16) -> u16` | Low byte of x (x & 0xFF). |
| `reverse_bits` | `run(x: u16) -> u16` | Reverse the 16 bits of x (bit 0 <-> bit 15, ...). |
| `rotl16` | `run(x: u16, n: u16) -> u16` | Rotate the 16 bits of x left by n (n taken mod 16). |
| `rotr16` | `run(x: u16, n: u16) -> u16` | Rotate the 16 bits of x right by n (n taken mod 16). |
| `swap_bytes` | `run(x: u16) -> u16` | Swap the high and low bytes of x ((x << 8) \| (x >> 8)). |
| `trailing_zeros` | `run(x: u16) -> u16` | Count of trailing (low) zero bits in the 16-bit value (16 for x == 0). |

## bit-mask (11)

| id | signature | summary |
|---|---|---|
| `bit_is_set` | `run(x: u16, bit: u16) -> u16` | Returns 1 if bit number `bit` of x is set, else 0. |
| `clear_bit` | `run(x: u16, bit: u16) -> u16` | Clear bit number `bit` of x to 0. |
| `mask_has_all` | `run(x: u16, mask: u16) -> u16` | Returns 1 if x has ALL bits of mask set: (x & mask) == mask. |
| `mask_has_any` | `run(x: u16, mask: u16) -> u16` | Returns 1 if x has ANY bit of mask set: (x & mask) != 0. |
| `mask_intersection` | `run(a: u16, b: u16) -> u16` | Intersection of two bit masks: a & b (bits set in both). |
| `mask_union` | `run(a: u16, b: u16) -> u16` | Union of two bit masks: a \| b (every bit set in either). |
| `mask_xor` | `run(a: u16, b: u16) -> u16` | Symmetric difference of two bit masks: a ^ b (bits set in exactly one). |
| `parity` | `run(x: u16) -> u16` | Parity: 1 if the number of set bits is odd, else 0. |
| `popcount` | `run(x: u16) -> u16` | Population count: the number of set bits in a 16-bit value. |
| `set_bit` | `run(x: u16, bit: u16) -> u16` | Set bit number `bit` of x to 1. |
| `toggle_bit` | `run(x: u16, bit: u16) -> u16` | Toggle (flip) bit number `bit` of x. |

## bounds (6)

| id | signature | summary |
|---|---|---|
| `between_exclusive` | `run(x: u16, lo: u16, hi: u16) -> u16` | Returns 1 if lo < x < hi (strictly inside, exclusive bounds), else 0. |
| `clamp` | `run(x: u16, lo: u16, hi: u16) -> u16` | Clamp a value to the inclusive range [lo, hi]. |
| `normalize_0_100` | `run(x: u16, lo: u16, hi: u16) -> u16` | Rescale x within [lo, hi] to a 0..100 percentage (clamped; 0 if hi <= lo). |
| `round_to_multiple` | `run(x: u16, step: u16) -> u16` | Round x to the NEAREST multiple of step (ties up; x if step == 0). |
| `snap_down` | `run(x: u16, step: u16) -> u16` | Round x DOWN to the nearest multiple of step (x if step == 0). Floor to grid. |
| `snap_up` | `run(x: u16, step: u16) -> u16` | Round x UP to the nearest multiple of step (x if step == 0). Ceil to grid. |

## bucket-convert (3)

| id | signature | summary |
|---|---|---|
| `bucket3` | `run(x: u16, t1: u16, t2: u16) -> u16` | Bucket x into 0, 1, or 2 by two ascending thresholds: x<t1 → 0, x<t2 → 1, else 2. |
| `byte_to_percent` | `run(b: u16) -> u16` | Convert a 0..255 byte scale to a 0..100 percent: b*100/255. |
| `percent_to_byte` | `run(p: u16) -> u16` | Convert a 0..100 percent to a 0..255 byte scale: p*255/100. |

## calendrical-checksum (4)

| id | signature | summary |
|---|---|---|
| `day_of_week` | `run(year: u16, month: u16, day: u16) -> u16` | Day of week for a Gregorian date via Zeller's congruence: 0=Saturday, 1=Sunday, 2=Monday, ... 6=Friday. |
| `days_in_month` | `run(month: u16, is_leap: u16) -> u16` | Number of days in a month (1-12; 0 for an invalid month), given a leap-year flag for February. |
| `is_leap_year` | `run(year: u16) -> u16` | Returns 1 if year is a Gregorian leap year, else 0: divisible by 4, except centuries not divisible by 400. |
| `luhn_check` | `run(n: u16) -> u16` | Returns 1 if n's decimal digits pass the Luhn checksum (mod 10, doubling every second digit from the right), else 0. |

## checked-arithmetic (28)

| id | signature | summary |
|---|---|---|
| `abs_diff_u32` | `AbsDiffWide::run() -> u16` | Absolute difference \|a - b\| between two wide u32 values — the exact wide sibling of abs_diff (which works over u16 and can't represent differences beyond 65535). |
| `add3_checked_u32` | `Add3Checked::run() -> u16` | Checked three-way add at u32: a+b+c, escalating if either add step overflows — the exact, wide sibling of sum3 (which saturates at u16). |
| `add_checked_u32` | `AddChecked::run() -> u16` | Checked u32 add: escalates (needs_wider_math) instead of silently wrapping if a + b overflows u32. |
| `avg2_u32` | `Avg2Wide::run() -> u16` | Average of two wide u32 values, (a + b) / 2, computed without overflow — the wide sibling of avg2 (which works over u16). |
| `clamp_u32` | `ClampWide::run() -> u16` | Clamp a wide u32 value to the inclusive range [lo, hi] — the wide sibling of clamp (which works over u16). |
| `div_ceil_u32` | `DivCeil::run() -> u16` | Ceiling division of two u32 values: the smallest integer >= a / b. Escalates (needs_wider_math) if b is zero. |
| `div_exact_u32` | `DivExact::run() -> u16` | Exact u32 division: escalates (needs_wider_math) if b is zero or a doesn't divide evenly by b — a wrong-plan signal for word problems that declared an exact division. |
| `div_floor_u32` | `DivFloor::run() -> u16` | Floor division of two u32 values: a / b, rounded down. Escalates (needs_wider_math) if b is zero. |
| `divides_u32` | `DividesWide::run() -> u16` | Returns 1 if a divides b evenly at wide u32 width (b % a == 0, a != 0), else 0 — the wide sibling of divides (which works over u16). |
| `fits_u16` | `FitsU16::run() -> u16` | Returns 1 if a wide u32 value fits in u16 (<= 65535) without narrowing loss, else 0. |
| `gcd_u32` | `GcdWide::run() -> u16` | Greatest common divisor of two wide u32 values via an inline Euclidean loop — the wide sibling of gcd (which works over u16 and can't represent divisors beyond 65535). |
| `lcm_u32` | `LcmChecked::run() -> u16` | Least common multiple of two wide u32 values via an inline GCD (0 if either is 0), escalating on overflow — unlike lcm (u16, silently wraps on overflow), this is the exact, checked wide sibling. |
| `max_u32` | `MaxWide::run() -> u16` | Maximum of two wide u32 values — the exact wide sibling of max (which works over u16). |
| `min_u32` | `MinWide::run() -> u16` | Minimum of two wide u32 values — the exact wide sibling of min (which works over u16). |
| `mod_u32` | `ModU32::run() -> u16` | Remainder of two u32 values: a % b. Escalates (needs_wider_math) if b is zero. |
| `mul3_checked_u32` | `Mul3Checked::run() -> u16` | Checked three-way multiply at u32: a*b*c, escalating if either multiply step overflows (e.g. a box volume: length*width*height). |
| `mul_add_checked_u32` | `MulAddChecked::run() -> u16` | Checked fused multiply-add at u32: a*b+c, escalating on either the multiply or the add overflowing (e.g. a per-unit price times a quantity, plus a flat fee). |
| `mul_checked_u32` | `MulChecked::run() -> u16` | Checked u32 multiply: escalates (needs_wider_math) instead of wrapping if a * b overflows u32. |
| `mul_sub_checked_u32` | `MulSubChecked::run() -> u16` | Checked fused multiply-subtract at u32: a*b-c, escalating if the multiply overflows or c exceeds the product (e.g. a per-unit price times a quantity, minus a flat discount). |
| `mul_u16_u16_to_u32` | `MulWide::run() -> u16` | Exact product of two u16 values as a wide u32 (never overflows: 65535*65535 fits u32). The math-campaign foundation cell — most checked arithmetic composes from this. |
| `pow_checked_u32` | `PowChecked::run() -> u16` | Checked exact exponentiation at u32: base^exp, escalating the moment a multiply step would overflow (distinct from pow_small, which saturates at u16 — this stays exact or hands off). 0^0 = 1. |
| `range_check_u32` | `RangeCheckWide::run() -> u16` | Returns 1 if lo <= x <= hi at wide u32 width, else 0 — the wide sibling of range_check (which works over u16). |
| `smag_add` | `SmagAdd::run() -> u16` | Sign-magnitude add: combine two signed quantities represented as (magnitude, sign) pairs — neg_a/neg_b are 0 (nonnegative) or 1 (negative), since the dialect has no i32 and this is how the math-campaign renderer tracks signed differences at u32 width (docs/math-campaign-spec.md). Escalates on magnitude overflow. |
| `smag_cmp` | `SmagCmp::run() -> u16` | Compare two signed quantities represented as (magnitude, sign) pairs (neg 0=nonnegative, 1=negative, per smag_add): 0 if a < b, 1 if equal, 2 if a > b — the sign-magnitude counterpart of frac_cmp's ordering-code convention. |
| `smag_div` | `SmagDiv::run() -> u16` | Divide two signed values exactly: magnitudes divide (escalating on a nonzero remainder), sign is same-positive/different-negative (per smag_add). |
| `smag_mul` | `SmagMul::run() -> u16` | Multiply two signed values: magnitudes multiply (checked for overflow), sign is same-positive/different-negative (per smag_add). |
| `smag_sub` | `SmagSub::run() -> u16` | Sign-magnitude subtract: a - b for two signed quantities represented as (magnitude, sign) pairs (neg 0=nonnegative, 1=negative, per smag_add) — computed by flipping b's sign and adding, the same rule table as smag_add. Escalates on magnitude overflow. |
| `sub_checked_u32` | `SubChecked::run() -> u16` | Checked u32 subtract: escalates (needs_wider_math) instead of wrapping if b > a (the result would be negative). |

## combinatorics (12)

| id | signature | summary |
|---|---|---|
| `bell_number` | `BellNumber::run() -> u16` | The nth Bell number B_n (the number of ways to partition an n-element set): 1, 1, 2, 5, 15, 52, 203, 877, 4140, ... Computed via the Bell triangle, kept in one array updated in place (each new row's first entry is the previous row's last entry; each subsequent entry is the running sum plus the entry above it) -- checked, escalates instead of silently wrapping once an intermediate row sum would exceed u32::MAX. |
| `catalan_number` | `CatalanNumber::run() -> u16` | The nth Catalan number (C(0)=1, C(n+1) = C(n)*2*(2n+1)/(n+2) — an exact recurrence, each step's division always lands evenly), checked: escalates on overflow rather than silently wrapping. Note the recurrence's own pre-division intermediate can overflow u32 before the true Catalan number itself would (the same class of limitation choose_u32 documents) — verified safe through C(17); beyond that, escalation is possible even though C(18)/C(19) themselves would still fit u32. |
| `choose_u32` | `ChooseWide::run() -> u16` | Binomial coefficient "n choose k" (nCr), checked: the count of k-element subsets of an n-element set, via the multiplicative running-division formula (each step's quotient is always exact, but the pre-division product can transiently exceed the final answer, so this escalates somewhat before n choose k itself would overflow u32 — a known limitation of single-pass 32-bit intermediates, not a false claim). Escalates rather than silently wrapping. |
| `derangement_count` | `DerangementCount::run() -> u16` | The nth derangement number (D(0)=1, D(1)=0, D(n)=(n-1)*(D(n-1)+D(n-2)) — the count of permutations of n items with no fixed point), checked: escalates instead of silently wrapping once D(n) would exceed u32::MAX (n >= 14). Unlike catalan_number's recurrence, this one's intermediate never overflows before the true result itself would (verified) — the multiplier grows linearly (n-1) against a linearly-combined sum, not against an already-exponential value. |
| `factorial_checked_u32` | `FactorialChecked::run() -> u16` | Factorial of n, checked: n! — escalates instead of silently wrapping once n! would exceed u32::MAX (n >= 13, since 13! overflows u32). |
| `fibonacci_checked_u32` | `FibonacciChecked::run() -> u16` | The nth Fibonacci number (F(0)=0, F(1)=1, F(n)=F(n-1)+F(n-2)), checked: escalates instead of silently wrapping once F(n) would exceed u32::MAX (n >= 47). |
| `is_catalan_number` | `run(x: u16) -> u16` | Check whether x is a Catalan number (1, 1, 2, 5, 14, 42, 132, 429, ...) -- the inverse-membership test, distinct from catalan_number (which computes the nth one directly). Walks the same recurrence catalan_number uses (C(0)=1, C(n+1)=C(n)*2*(2n+1)/(n+2)) upward until it reaches or passes x, bounded by x itself. Never escalates: x is u16-bounded, and C(12) = 208012 already exceeds u16::MAX, so the search always terminates within the u16 domain long before any u32 intermediate could overflow. |
| `lucas_u_v` | `LucasUV::run() -> u16` | Generalized Lucas sequence pair U_n/V_n for parameters p, q (both non-negative): U(0)=0, U(1)=1, U(n)=p*U(n-1)+q*U(n-2); V(0)=2, V(1)=p, V(n)=p*V(n-1)+q*V(n-2) -- both share one recurrence structure, so one cell computes them together. p=2,q=1 gives the Pell numbers (U) and companion Pell / Pell-Lucas numbers (V) -- pell_number and pell_lucas_number are not shipped as separate cells for exactly that reason. p=1,q=1 reproduces fibonacci_checked_u32 (U) and the classic Lucas numbers (V); fibonacci_checked_u32 stays its own cell for its own retrieval identity, not folded away, the same precedent triangular/polygonal_number(3,n) already set. |
| `permute_u32` | `PermuteWide::run() -> u16` | Permutations "n pick k" (nPr): the count of ordered k-element selections from an n-element set, n!/(n-k)! computed directly as a product of k descending terms (never materializing the full factorials). Escalates on overflow rather than silently wrapping. |
| `stirling_first` | `StirlingFirst::run() -> u16` | Unsigned Stirling number of the first kind c(n, k): the number of permutations of n elements with exactly k cycles. (The signed convention s(n,k) = (-1)^(n-k) * c(n,k) is not used here -- c(n,k) is always non-negative, avoiding a sign-magnitude return for a cell whose whole job is counting.) Computed via the standard recurrence c(n,k) = (n-1)*c(n-1,k) + c(n-1,k-1), kept in one array and updated in place row by row (the same in-place carry technique bell_number uses, since this recurrence also needs both the just-written and the about-to-be-overwritten value at once). |
| `stirling_second` | `StirlingSecond::run() -> u16` | Stirling number of the second kind S(n, k): the number of ways to partition an n-element set into exactly k non-empty subsets. Computed via the inclusion-exclusion closed form S(n,k) = (1/k!) * sum_{j=0}^{k} (-1)^(k-j) * C(k,j) * j^n -- the alternating sum tracked as a sign-magnitude pair (no array needed), C(k,j) via the same multiplicative running-division formula choose_u32 uses, then divided exactly by k! at the end. |
| `tribonacci_number` | `TribonacciChecked::run() -> u16` | The nth Tribonacci number (T(0)=0, T(1)=1, T(2)=1, T(n)=T(n-1)+T(n-2)+T(n-3)), checked: escalates instead of silently wrapping once T(n) would exceed u32::MAX. Distinct from fibonacci_checked_u32's two-term recurrence -- a genuinely different sequence, not reducible to lucas_u_v's two-term p/q family. |

## distance (4)

| id | signature | summary |
|---|---|---|
| `abs_diff` | `run(a: u16, b: u16) -> u16` | Absolute difference \|a - b\| between two values. |
| `chebyshev` | `Pts::run() -> u16` | Chebyshev (chessboard) distance between two grid points: max(\|dx\|, \|dy\|). |
| `euclid_sq` | `Pts::run() -> u16` | Squared Euclidean distance between two grid points: dx*dx + dy*dy (no sqrt). Wide u32 dist field. |
| `manhattan` | `Pts::run() -> u16` | Manhattan distance between two grid points (typed state). |

## fixed-point (5)

| id | signature | summary |
|---|---|---|
| `q_div` | `run(a: u16, b: u16) -> u16` | Q8.8 fixed-point divide: (a << 8) / b, returning 0 when b == 0 (no divide-by-zero). |
| `q_lerp` | `run(a: u16, b: u16, t: u16) -> u16` | Linear interpolation from a to b by t (Q0.8 fraction, 0..256 = 0.0..1.0): a + (b-a)*t/256. Also an EMA step: q_lerp(prev, sample, alpha). |
| `q_mul` | `run(a: u16, b: u16) -> u16` | Q8.8 fixed-point multiply: (a * b) >> 8, computed wide so the 16.16 intermediate doesn't overflow. |
| `q_sigmoid` | `run(x: i16) -> u16` | Q8.8 fixed-point "hard sigmoid": a well-known piecewise-linear stand-in for the true sigmoid, clamp(x/4 + 0.5, 0, 1) — exact at x=0, saturating to 0/1 outside roughly [-4, 4], monotonic and cheap everywhere between. Input is signed (Q8.8, negative values meaningful, e.g. -256 = -1.0); output is unsigned Q8.8 in [0, 256] (0.0 to 1.0). q_tanh is deliberately not a separate cell: the same derivation (tanh(x) = 2*sigmoid(2x)-1) reduces to clamp_i16(x, -256, 256) exactly, already covered by that cell's own tags. |
| `q_sqrt` | `run(x: u16) -> u16` | Q8.8 fixed-point square root: sqrt(x/256)*256, via a branch-free bitwise integer square root on the widened x*256 (u32 only as a local, never a call param/return — the pattern every Q8.8 free function follows). A naive linear-scan integer sqrt was tried first and cost 3.6M cycles at the domain extreme (past the 2,000,000 default); this bitwise version costs under 20,000. |

## fractions (22)

| id | signature | summary |
|---|---|---|
| `frac_add` | `FracAdd::run() -> u16` | Add two fractions na/da + nb/db, reduced to lowest terms via the shared gcd_u32 kernel. |
| `frac_add_whole` | `FracAddWhole::run() -> u16` | Add a whole number to a fraction: n/d + whole = (n + whole*d)/d, reduced to lowest terms via the shared gcd_u32 kernel. |
| `frac_avg2` | `FracAvg2::run() -> u16` | Average of two fractions na/da and nb/db, reduced to lowest terms via the shared gcd_u32 kernel. |
| `frac_cmp` | `FracCmp::run() -> u16` | Compare two fractions na/da vs nb/db via cross-multiplication (works on unreduced fractions, e.g. 1/2 vs 2/4): 0 if less, 1 if equal, 2 if greater. |
| `frac_div` | `FracDiv::run() -> u16` | Divide two fractions (na/da) / (nb/db) = (na*db)/(da*nb), reduced to lowest terms via the shared gcd_u32 kernel. |
| `frac_eq` | `FracEq::run() -> u16` | Returns 1 if two fractions na/da and nb/db are equal, else 0 — via cross-multiplication, so unreduced-but-equivalent fractions (e.g. 1/2 vs 2/4) still compare equal without needing to reduce first. |
| `frac_is_proper` | `FracIsProper::run() -> u16` | Returns 1 if a fraction n/d is proper (n < d, i.e. less than one whole), else 0. Escalates (halt 0xFF06, out_of_domain) if d == 0. |
| `frac_max` | `FracMax::run() -> u16` | The larger of two fractions na/da and nb/db, by cross-multiplication (works on unreduced fractions) — returns its numerator/denominator as given (ties keep na/da). Distinct from frac_cmp, which only returns an ordering code, not the winning fraction itself. |
| `frac_min` | `FracMin::run() -> u16` | The smaller of two fractions na/da and nb/db, by cross-multiplication (works on unreduced fractions) — returns its numerator/denominator as given (ties keep na/da). Distinct from frac_cmp, which only returns an ordering code, not the winning fraction itself. |
| `frac_mul` | `FracMul::run() -> u16` | Multiply two fractions na/da * nb/db, reduced to lowest terms via the shared gcd_u32 kernel. |
| `frac_of_whole` | `FracOfWhole::run() -> u16` | A fraction of a whole number, computed exactly: n/d * whole, escalating if it doesn't divide evenly (a wrong-plan signal — e.g. "3/4 of 20" should be exact for a grade-school word problem) or if the multiply overflows. |
| `frac_of_whole_floor` | `FracOfWholeFloor::run() -> u16` | A fraction of a whole number, rounded down: floor(n/d * whole) — the floor sibling of frac_of_whole (which escalates if the result isn't exact). Never escalates on an inexact split (e.g. "90% of 23" is a real, non-exact GSM8K-style shape, unlike "3/4 of 20"); still escalates if the multiply overflows. |
| `frac_reciprocal` | `FracReciprocal::run() -> u16` | Reciprocal of a fraction n/d: swaps to d/n. Escalates (halt 0xFF06, out_of_domain) if n == 0 (a zero fraction has no reciprocal) or d == 0 (not a valid fraction to begin with). |
| `frac_reduce` | `FracReduce::run() -> u16` | Reduce a fraction n/d to lowest terms via the shared gcd_u32 kernel — a two-u32-param call (first arg rides HL:DE, second rides the stack; docs 10 §Calls), so the Euclidean loop lives once in the prelude instead of inlined in every fraction cell. |
| `frac_scale` | `FracScale::run() -> u16` | Scale a fraction by an integer: (n/d) * k, reduced to lowest terms via the shared gcd_u32 kernel — unlike frac_of_whole (which requires an exact whole-number result), this always stays a fraction. |
| `frac_sub` | `FracSub::run() -> u16` | Subtract two fractions na/da - nb/db, reduced to lowest terms via the shared gcd_u32 kernel. |
| `frac_sub_from_whole` | `FracSubFromWhole::run() -> u16` | Subtract a fraction from a whole number: whole - n/d, reduced to lowest terms via the shared gcd_u32 kernel. |
| `frac_to_mixed` | `FracToMixed::run() -> u16` | Convert an improper fraction n/d to a mixed number: whole + num/den, where the remaining fraction is reduced to lowest terms via the shared gcd_u32 kernel (num=0, den=1 if n divides evenly by d). |
| `is_integer` | `IsInteger::run() -> u16` | Returns 1 if the wide fraction n/d is a whole number (n divides evenly by d), else 0 — a wrong-plan signal for word problems that expect an exact split. |
| `mixed_to_frac` | `MixedToFrac::run() -> u16` | Convert a mixed number (whole + num/den) to a single improper fraction: n = whole*den + num, d = den — the exact inverse of frac_to_mixed. |
| `ratio_split2` | `RatioSplit2::run() -> u16` | Split a wide total into two parts in a given ratio (ratio_a : ratio_b): part_a = total*ratio_a/(ratio_a+ratio_b), part_b = total - part_a — guaranteed to sum exactly to total (the remainder from integer division always lands on part_b), unlike computing both parts independently. |
| `ratio_split3` | `RatioSplit3::run() -> u16` | Split a wide total three ways by a given ratio (ratio_a : ratio_b : ratio_c): part_a and part_b get their proportional share by integer division, part_c takes the remainder — guaranteed to sum exactly to total (the direct 3-way sibling of ratio_split2). |

## geometry (6)

| id | signature | summary |
|---|---|---|
| `cos_frac_from_sides` | `CosFracFromSides::run() -> u16` | Cosine of the angle opposite side c in a triangle with integer sides (a, b, c), via the law of cosines rearranged to an exact fraction: cos C = (a² + b² − c²) / (2ab) — no square root, no trig, just integer arithmetic. Returned as a sign-magnitude fraction (mag_num, neg_num, den) since the numerator is negative whenever angle C is obtuse; reduced to lowest terms via the shared gcd_u32 kernel. |
| `geom_distance_3d` | `GeomDistance3d::run() -> u16` | Squared Euclidean distance between two 3D points -- the missing 3D sibling of euclid_sq, which stays squared for the same reason euclid_sq does (no square root in the dialect). Each signed coordinate difference is computed via an excess-32768 shift (mapping i16's range onto u16 losslessly) feeding the shared iabs_diff kernel, so no i16 subtraction ever risks overflowing i16's own range. |
| `heron_16a2` | `Heron16A2::run() -> u16` | 16 times the squared area of a triangle with integer sides (a, b, c), via Heron's formula rearranged to avoid square roots entirely: 16·Area² = (a+b+c)(−a+b+c)(a−b+c)(a+b−c). Always a non-negative integer for a valid triangle — comparable, summable, and equality-testable without ever taking a root. |
| `shoelace_area_x2` | `ShoelaceAreaX2::run() -> u16` | Twice the area of a triangle from three integer vertices (x1,y1),(x2,y2),(x3,y3), via the shoelace formula: \|x1*(y2-y3) + x2*(y3-y1) + x3*(y1-y2)\| — always an integer, unlike the raw area (which is a half-integer for e.g. a right triangle with legs 1 and 1). Coordinates are unsigned; the three (y-difference)*(x-coordinate) terms are combined as sign-magnitude values inline (no shared smag_* subroutine call — a u32 value still can't cross more than one call boundary), since a term or the running sum can go negative before the final absolute value. |
| `shoelace_area_x2_quad` | `ShoelaceAreaX2Quad::run() -> u16` | Twice the area of a quadrilateral from four integer vertices (x1,y1)..(x4,y4), generalizing shoelace_area_x2's triangle formula to \|x1*(y2-y4) + x2*(y3-y1) + x3*(y4-y2) + x4*(y1-y3)\| — always an integer. Coordinates are unsigned; the four (y-difference)*(x-coordinate) terms are combined as sign-magnitude values inline (no shared smag_* subroutine call — a u32 value still can't cross more than one call boundary), the same pattern shoelace_area_x2 uses, extended to a fourth term. |
| `triangle_is_valid` | `run(a: u16, b: u16, c: u16) -> u16` | Returns 1 if three side lengths (a, b, c) form a valid (non-degenerate) triangle, i.e. each side is strictly less than the sum of the other two, else 0. Sums are widened to u32 internally so a large pair (e.g. two sides near 65535) can't wrap past u16 and silently flip the verdict. |

## hashing (4)

| id | signature | summary |
|---|---|---|
| `crc8_step` | `run(crc: u16, byte: u16) -> u16` | One CRC-8 (Dallas/Maxim, poly 0x8C reflected) step over a byte. |
| `fnv1a_step` | `run(hash: u16, byte: u16) -> u16` | One FNV-1a-style hash step over a byte: (hash ^ byte) * prime (16-bit). |
| `hash_pair` | `run(a: u16, b: u16) -> u16` | Deterministic hash mixing two values into one u16. |
| `mix16` | `run(x: u16) -> u16` | Avalanche-mix one u16 into a well-scrambled u16 (a finalizer / hash of one value). |

## matrix (2)

| id | signature | summary |
|---|---|---|
| `matrix_det_2x2` | `MatrixDet2x2::run() -> u16` | Determinant of a 2x2 matrix [[a, b], [c, d]]: a*d - b*c. Signed result tracked as a (magnitude, sign) pair, the same technique the vector pack's cross_product/triple_scalar_product use -- the "vector floor" exception to the matrix non-goal extends this far and no further (see docs/library-growth.md). |
| `matrix_solve_2x2` | `MatrixSolve2x2::run() -> u16` | Solve a 2x2 linear system [[a, b], [c, d]] * [x, y] = [e, f] via Cramer's rule, returning x and y as exact signed fractions sharing one positive denominator (det, normalized positive by flipping both numerators' signs if the raw determinant was negative) -- matrix_det_2x2's own formula computes that shared denominator's magnitude and sign before this cell reuses it inline. |

## money-bps (8)

| id | signature | summary |
|---|---|---|
| `bps_decrease_between` | `BpsDecreaseBetween::run() -> u16` | Infer the basis-points decrease between two wide values: given before and after (after <= before), the rate = (before - after) * 10000 / before — the inverse of decrease_by_bps (that computes the final value from a rate; this recovers the rate from the two values). |
| `bps_increase_between` | `BpsIncreaseBetween::run() -> u16` | Infer the basis-points increase between two wide values: given before and after (after >= before), the rate = (after - before) * 10000 / before — the inverse of increase_by_bps (that computes the final value from a rate; this recovers the rate from the two values). |
| `bps_of` | `BpsOf::run() -> u16` | Basis points of a wide value: value * bps / 10000 (e.g. 500 bps of 1000 is 50 — 5%). Escalates (needs_wider_math) on multiply overflow. |
| `cents_mul_qty` | `CentsMulQty::run() -> u16` | Total price in cents (the minor unit of any decimal currency — cents, pence, kopecks, not USD specifically): unit_cents * qty. Escalates (needs_wider_math) on multiply overflow — distinct from mul_u16_u16_to_u32 (that one always fits u32 exactly; this one's unit_cents is already wide and can genuinely overflow). |
| `decrease_by_bps` | `DecreaseByBps::run() -> u16` | Decrease a wide value by bps basis points (covers discount: value - value*bps/10000). Escalates if the discount would exceed the value, or on multiply overflow. |
| `increase_by_bps` | `IncreaseByBps::run() -> u16` | Increase a wide value by bps basis points (covers tax/tip/markup — same formula: value + value*bps/10000). Escalates on multiply or add overflow. |
| `original_before_bps_decrease` | `OriginalBeforeDecrease::run() -> u16` | Recover the original value before a bps decrease, given the final value: final * 10000 / (10000 - bps). The inverse of decrease_by_bps. |
| `original_before_bps_increase` | `OriginalBeforeIncrease::run() -> u16` | Recover the original value before a bps increase, given the final value: final * 10000 / (10000 + bps). The inverse of increase_by_bps. |

## number-theory (52)

| id | signature | summary |
|---|---|---|
| `big_omega` | `run(n: u16) -> u16` | Omega(n): total count of prime factors of n counted with multiplicity (n >= 1; Omega(1) = 0) -- distinct from little_omega (counts distinct primes only) and factor_count (counts divisors, not prime factors). |
| `carmichael_lambda` | `run(n: u16) -> u16` | The Carmichael function lambda(n): the exponent of the multiplicative group mod n -- the smallest m such that a^m == 1 (mod n) for every a coprime to n. Computed as the lcm of lambda(p^e) over each prime-power factor of n: lambda(2)=1, lambda(4)=2, lambda(2^e)=2^(e-2) for e>=3; lambda(p^e)=(p-1)*p^(e-1) for odd p (the same formula euler_totient uses at odd prime powers). Every intermediate lcm combination is itself a divisor of the final lambda(n), which is always <= n -- so despite computing at u32 width for safety, nothing in the u16 input domain can actually overflow it (proven, not just unobserved). |
| `centered_polygonal_number` | `run(s: u16, n: u16) -> u16` | The nth centered s-gonal number: C(s, n) = 1 + s*n*(n+1)/2 — the center point plus n rings of s points each (s >= 3, n >= 0; n=0 is the bare center point, 1, for every s). Star numbers are this family's s=12 case one ring later than its own usual 1-indexed convention (star_number(k) = centered_polygonal_number(12, k-1)) — not shipped as a separate cell for exactly that reason. |
| `crt_solve_pair` | `CrtSolvePair::run() -> u16` | Chinese Remainder Theorem for two congruences: the unique x in [0, m1*m2) with x == r1 (mod m1) and x == r2 (mod m2), when m1 and m2 are coprime. Computes the inverse of m1 modulo m2 via an inlined extended Euclidean algorithm (the same one mod_inverse uses — duplicated here rather than called, since a u32 value still can't cross more than one call boundary), then combines it with the standard closed-form x = r1 + m1*((r2-r1)*inv(m1, m2) mod m2). |
| `cube_sat` | `run(n: u16) -> u16` | Saturating cube: n*n*n, capped at 65535 (n >= 41 saturates). |
| `digit_product` | `run(n: u16) -> u16` | Product of the decimal digits of n (0 has product 0, its only digit). |
| `digit_reverse` | `run(n: u16) -> u16` | Reverse the decimal digits of n (e.g. 123 -> 321; trailing zeros drop, so 120 -> 21). |
| `digit_sum` | `run(n: u16) -> u16` | Sum of the decimal digits of n. |
| `digital_root` | `run(n: u16) -> u16` | Digital root: repeatedly sum the decimal digits of n until a single digit remains, computed via the exact closed form (1 + (n-1) mod 9, 0 for n == 0) rather than iterating -- distinct from digit_sum (one summing pass) and persistent_digital_root (which counts the iterations this cell short-circuits). |
| `discrete_log_naive` | `DiscreteLogNaive::run() -> u16` | Discrete logarithm by brute-force search: the smallest k in [0, max_exp) with base^k == target (mod m). Genuinely bounded by the caller-supplied max_exp (unlike a general discrete-log solve, which is believed hard) -- a plan verifier's "does this exponent exist within a reasonable search window" check. |
| `divides` | `run(a: u16, b: u16) -> u16` | Returns 1 if a divides b evenly (b % a == 0, a != 0), else 0. |
| `divisor_power_sum` | `DivisorPowerSum::run() -> u16` | sigma_k(n): sum of the k-th powers of the positive divisors of n (n >= 1) -- generalizes factor_count (k=0, counts divisors) and sum_divisors (k=1, sums them) with an explicit exponent, the same general-parameter-sibling shape weighted_sum2 already gives weighted_sum. |
| `euler_totient` | `run(n: u16) -> u16` | Euler's totient (phi): count of integers in [1, n] coprime to n (n >= 1; phi(1) = 1 by convention). |
| `extended_gcd` | `ExtendedGcd::run() -> u16` | Extended Euclidean algorithm: gcd(a, b) plus the Bezout coefficients x, y with a*x + b*y == gcd(a, b). mod_inverse and crt_solve_pair each inline one Bezout chain internally already (a u32 value still can't cross more than one call boundary, so there's no shared subroutine to call) -- this is the standalone two-chain version those two only compute half of. |
| `factor_count` | `run(n: u16) -> u16` | Number of positive divisors of n (0 for n == 0). |
| `gcd` | `run(a: u16, b: u16) -> u16` | Greatest common divisor (Euclid's algorithm). |
| `gcd3` | `run(a: u16, b: u16, c: u16) -> u16` | Greatest common divisor of three values. |
| `is_automorphic_number` | `run(n: u16) -> u16` | Check whether n^2 ends with the decimal digits of n itself (e.g. 5^2=25, 6^2=36, 25^2=625, 76^2=5776) -- a classic "self-reproducing" number check. Computed exactly via n*n mod 10^(digit count of n), so no string/digit-array comparison is needed. |
| `is_coprime` | `run(a: u16, b: u16) -> u16` | Returns 1 if a and b are coprime (gcd == 1), else 0. |
| `is_palindromic_number` | `run(n: u16, base: u16) -> u16` | Check whether n is palindromic when written in the given base (base >= 2) -- its digits read the same forwards and backwards. Computed by reversing n's base-b digits (the same trick digit_reverse uses at base 10) and comparing to the original, rather than building a digit array. |
| `is_polygonal_number` | `run(s: u16, x: u16) -> u16` | Check whether x is an s-gonal (polygonal) number for a given side count s (s >= 3) -- is there some n >= 0 with polygonal_number(s, n) == x. The membership-test counterpart of polygonal_number, one general predicate instead of a separate fixed-s check for every side count. |
| `is_pow2` | `run(x: u16) -> u16` | Returns 1 if x is a power of two, else 0. |
| `is_prime` | `run(n: u16) -> u16` | Returns 1 if n is prime, else 0. |
| `is_prime_u32` | `IsPrimeWide::run() -> u16` | Returns 1 if n is prime at wide u32 width, else 0 — the wide sibling of is_prime (which works over u16, up to 65535). Trial division scales with sqrt(n): a large prime near u32::MAX needs on the order of tens of millions of cycles, far past the 2,000,000 default — pass a larger --cycles budget explicitly for n much beyond a few million. |
| `is_quadratic_residue` | `run(x: u16, p: u16) -> u16` | Check whether x is a quadratic residue mod p: does some y in [0, p) satisfy y*y == x (mod p)? Works for any modulus p >= 2, not just primes, via direct search over every residue -- so cost scales with p (like is_prime_u32; budget a larger --cycles for p much beyond a few thousand). |
| `is_repdigit` | `run(n: u16) -> u16` | Check whether every decimal digit of n is the same digit (e.g. 4444, 555, 22 -- and trivially any single digit 0-9). Distinct from is_palindromic_number: a repdigit is always a palindrome but not vice versa (121 is palindromic, not a repdigit). |
| `is_square` | `run(n: u16) -> u16` | Returns 1 if n is a perfect square, else 0. |
| `isqrt` | `run(n: u16) -> u16` | Integer square root: the largest r with r*r <= n. |
| `jacobi_symbol` | `run(a: u16, n: u16) -> i16` | The Jacobi symbol (a/n) for odd n > 0: 1 if a is a quadratic residue mod every prime factor of n (with multiplicity) an even number of times, -1 for an odd number of times, 0 if gcd(a, n) > 1. Computed by the standard law-of-quadratic-reciprocity reduction, tracking the sign as a parity flip (XOR) rather than a signed accumulator, since every intermediate value stays a plain nonnegative u16. |
| `jordan_totient` | `JordanTotient::run() -> u16` | Jordan's totient J_k(n): generalizes euler_totient with an exponent k (J_1(n) = phi(n)) -- the product over each prime-power factor p^e of n of p^((e-1)*k) * (p^k - 1). The (e-1)*k exponent is never computed as a scalar product (e up to ~15 times k up to 65535 would overflow u16 before any p^_ term is even reached) -- instead p^((e-1)*k) is built by repeatedly squaring the already-computed p^k value e-1 times, which stays small since e-1 is itself bounded (<= 15 in the u16 domain). |
| `lcm` | `run(a: u16, b: u16) -> u16` | Least common multiple of two values (a/gcd*b; 0 if either is 0). u16 domain. |
| `lcm3` | `run(a: u16, b: u16, c: u16) -> u16` | Least common multiple of three values. |
| `little_omega` | `run(n: u16) -> u16` | omega(n): count of distinct prime factors of n (n >= 1; omega(1) = 0 by convention) -- distinct from factor_count (counts divisors, not prime factors) and big_omega (counts prime factors with multiplicity, not distinct primes). |
| `mobius_function` | `run(n: u16) -> i16` | The Mobius function mu(n): 1 if n = 1, 0 if n has a squared prime factor (not squarefree), else (-1)^omega(n) for squarefree n (n >= 1). |
| `mod_add_u32` | `ModAddWide::run() -> u16` | Modular addition at wide u32 width: (a + b) mod m — reduces both operands mod m first, so a and b need not already be canonical residues. |
| `mod_inverse` | `ModInverse::run() -> u16` | Modular multiplicative inverse of a mod m: the x in [0, m) with a*x == 1 (mod m), via the iterative extended Euclidean algorithm. The Bezout coefficient tracked along the way can go negative, so it's carried as a sign-magnitude pair inline (no shared smag_* subroutine call — a u32 value still can't cross more than one call boundary), the same convention smag_add/pow_mod_u32 use. |
| `mod_mul_u32` | `ModMulWide::run() -> u16` | Modular multiplication at wide u32 width: (a * b) mod m — reduces both operands mod m first, then multiplies; the non-exponentiating sibling of pow_mod_u32, sharing its overflow bound. |
| `mod_sub_u32` | `ModSubWide::run() -> u16` | Modular subtraction at wide u32 width: (a - b) mod m, always returned in [0, m) — e.g. 3 - 5 mod 7 = 5, not a negative remainder. |
| `next_palindrome` | `run(n: u16) -> u16` | The smallest decimal palindrome strictly greater than n. Searches upward candidate by candidate (the worst-case gap within the u16 domain is 110, at n=1001 -- cheap); escalates if no palindrome exists at or below 65535 (true for n in roughly [65456, 65535], where reaching one would need a 6th digit). |
| `next_pow2` | `run(n: u16) -> u16` | Smallest power of two >= n (0 if it would exceed 65535; next_pow2(0) = 1). |
| `num_digits` | `run(n: u16) -> u16` | Number of decimal digits of n (0 has 1 digit). |
| `order_modulo` | `run(a: u16, n: u16) -> u16` | Multiplicative order of a mod n: the smallest k >= 1 with a^k == 1 (mod n). Requires gcd(a, n) == 1 (else no finite order exists) -- the order always divides euler_totient(n), so the search loop is bounded by n itself. |
| `persistent_digital_root` | `run(n: u16) -> u16` | Additive persistence: the number of digit-summing passes needed to reduce n to a single digit (0 if n is already a single digit) -- the step count, not the resulting digit itself. |
| `polygonal_number` | `run(s: u16, n: u16) -> u16` | The nth s-gonal (polygonal) number: P(s, n) = n + (s-2)*n*(n-1)/2, for a polygon with s sides (s >= 3). s=3 reproduces triangular's own values (kept as a separate cell for its own retrieval identity, not folded away), s=4 is the perfect squares, s=5 is pentagonal, s=6 is hexagonal, and so on — one general cell instead of a differently-named cell for every side count. |
| `pow_mod` | `run(base: u16, exp: u16, m: u16) -> u16` | Modular exponentiation: (base^exp) mod m (0 if m == 0). u16 domain m <= 256. |
| `pow_mod_u32` | `PowModWide::run() -> u16` | Modular exponentiation at wide u32 width: (base^exp) mod m — the wide sibling of pow_mod (u16 domain, m <= 256); lifts the modulus ceiling to 65536, wide enough for AIME's "find the remainder mod 1000" finishing move. Returns 0 if m == 0, matching pow_mod's convention. |
| `pow_small` | `run(base: u16, exp: u16) -> u16` | base raised to exp (saturating at 65535). 0^0 = 1. |
| `smallest_prime_factor` | `run(n: u16) -> u16` | Smallest prime factor of n (n >= 2) — the least prime p dividing n; returns n itself if n is prime. |
| `square_pyramidal_number` | `SquarePyramidal::run() -> u16` | The nth square pyramidal number: 1^2 + 2^2 + ... + n^2 = n*(n+1)*(2n+1)/6, checked — escalates instead of silently wrapping once the running sum would exceed u32::MAX. Computed by iterative summation rather than the closed form, so cost scales with n (like is_prime_u32, budget a larger --cycles for n much beyond a few thousand). |
| `sum_divisors` | `SumDivisors::run() -> u16` | Sum of the positive divisors of n (n >= 1), including 1 and n itself (sigma(n)) — the sum-valued sibling of factor_count (which counts divisors; this sums them, so it needs a wide result field since sigma(n) routinely exceeds 65535 within the u16 domain). |
| `triangular` | `run(n: u16) -> u16` | nth triangular number: 1+2+...+n = n*(n+1)/2 (overflow-safe; u16 domain n <= 361). |
| `triangular_inverse_exact` | `run(x: u16) -> u16` | Solve n*(n+1)/2 = x for n, the exact inverse of triangular: given a triangular number x, return which n produced it. Escalates if x isn't triangular (a wrong-plan signal, e.g. GSM8K's "how many rows" problems). Domain matches triangular's own (n <= 361, x <= 65341). |

## packing-bcd (4)

| id | signature | summary |
|---|---|---|
| `bcd_decode` | `run(bcd: u16) -> u16` | Decode a packed BCD byte (tens in the high nibble, units in the low nibble) back to its binary value. |
| `bcd_encode` | `run(n: u16) -> u16` | Encode a two-digit decimal value (0-99) as packed BCD: tens in the high nibble, units in the low nibble. |
| `pack_nibbles` | `run(hi: u16, lo: u16) -> u16` | Pack two 4-bit nibbles into one byte: (hi << 4) \| lo. Each input masked to its low nibble. |
| `pack_u8` | `run(hi: u16, lo: u16) -> u16` | Pack two byte values into one u16: (hi << 8) \| lo. Each input masked to its low byte, so out-of-range inputs stay defined. |

## percent (8)

| id | signature | summary |
|---|---|---|
| `discount_percent` | `run(value: u16, pct: u16) -> u16` | Decrease a value by pct percent: value - value*pct/100 (0 if pct >= 100). |
| `increase_percent` | `run(value: u16, pct: u16) -> u16` | Increase a value by pct percent: value + value*pct/100 (saturating at 65535). |
| `percent` | `run(part: u16, whole: u16) -> u16` | Percentage of a whole: part*100/whole, in 0..100+ (0 if whole == 0). |
| `permille` | `run(part: u16, whole: u16) -> u16` | Per-mille (parts per thousand): part*1000/whole (0 if whole == 0). |
| `ratio_255` | `run(part: u16, whole: u16) -> u16` | Ratio scaled to a 0..255 byte fraction: part*255/whole (0 if whole == 0). |
| `scale_percent` | `run(value: u16, pct: u16) -> u16` | Take pct percent of a value: value*pct/100. |
| `scale_percent_u32` | `ScalePercentWide::run() -> u16` | Take pct percent of a wide value: value*pct/100 at u32, escalating if the multiply overflows — the wide sibling of scale_percent, and the percent-of core the widened (u32) arithmetic lane resolves to. |
| `within_percent` | `run(actual: u16, target: u16, pct: u16) -> u16` | Returns 1 if actual is within pct percent of target (\|actual-target\|*100 <= target*pct). |

## physics (5)

| id | signature | summary |
|---|---|---|
| `clamp_f32` | `ClampF32::run() -> u16` | Clamp an f32 into [lo, hi] via max-then-min (x.max(lo).min(hi)) -- the branch-free form; NaN x resolves to lo (min/max treat NaN as missing data, the documented divergence from f32::clamp's NaN-propagating semantics), so the output is always a real bound. |
| `drag_force_f32` | `DragForce::run() -> u16` | Quadratic drag force k*v*\|v\| in IEEE binary32 -- signed (opposes the sign of v when the caller negates), correctly rounded per op through the owned softfloat kernels; a non-finite force escalates instead of flowing onward. |
| `kinetic_energy_f32` | `KineticEnergy::run() -> u16` | Kinetic energy 0.5*m*v*v in IEEE binary32 through the owned softfloat kernels -- correctly rounded per op, bit-identical to rustc f32; escalates float_overflow/float_domain instead of reporting a non-finite energy. |
| `spring_damper_step_f32` | `SpringDamperStep::run() -> u16` | One semi-implicit-Euler spring-damper step, IEEE binary32: a = -(k*x + c*v)*inv_m, then v' = v + a*dt and x' = x + v'*dt -- inverse mass as input, exactly how a Rapier-style engine stores it (and it keeps the cell division-free); non-finite state escalates instead of exploding the spring silently. |
| `verlet_step_f32` | `VerletStep::run() -> u16` | One position-Verlet step under constant acceleration, IEEE binary32: x' = x + v*dt + 0.5*a*dt*dt and v' = v + a*dt -- the integrator's arithmetic exactly as a Rapier-style f32 engine computes it, correctly rounded per op; non-finite results escalate instead of corrupting the trajectory. |

## predicates (14)

| id | signature | summary |
|---|---|---|
| `eq` | `run(a: u16, b: u16) -> u16` | Returns 1 if a == b, else 0. |
| `is_even` | `run(x: u16) -> u16` | Returns 1 if x is even, else 0. |
| `is_ge` | `run(a: u16, b: u16) -> u16` | Returns 1 if a >= b (at least), else 0. |
| `is_ge_u32` | `IsGeWide::run() -> u16` | Returns 1 if a >= b (at least) at wide u32 width, else 0 — the wide sibling of is_ge (which works over u16 and can't compare values beyond 65535, e.g. money totals in cents). |
| `is_gt` | `run(a: u16, b: u16) -> u16` | Returns 1 if a > b (strictly greater than), else 0. |
| `is_gt_u32` | `IsGtWide::run() -> u16` | Returns 1 if a > b (strictly greater than) at wide u32 width, else 0 — the wide sibling of is_gt (which works over u16 and can't compare values beyond 65535, e.g. money totals in cents). |
| `is_le` | `run(a: u16, b: u16) -> u16` | Returns 1 if a <= b (at most), else 0. |
| `is_le_u32` | `IsLeWide::run() -> u16` | Returns 1 if a <= b (at most) at wide u32 width, else 0 — the wide sibling of is_le (which works over u16 and can't compare values beyond 65535, e.g. money totals in cents). |
| `is_lt` | `run(a: u16, b: u16) -> u16` | Returns 1 if a < b (strictly less than), else 0. |
| `is_lt_u32` | `IsLtWide::run() -> u16` | Returns 1 if a < b (strictly less than) at wide u32 width, else 0 — the wide sibling of is_lt (which works over u16 and can't compare values beyond 65535, e.g. money totals in cents). |
| `is_odd` | `run(x: u16) -> u16` | Returns 1 if x is odd, else 0. |
| `is_zero` | `run(x: u16) -> u16` | Returns 1 if x is zero, else 0. |
| `neq` | `run(a: u16, b: u16) -> u16` | Returns 1 if a != b, else 0. |
| `nonzero` | `run(x: u16) -> u16` | Returns 1 if x is nonzero, else 0. |

## ranking-stats (16)

| id | signature | summary |
|---|---|---|
| `argmax3` | `run(a: u16, b: u16, c: u16) -> u16` | Index (0, 1, or 2) of the largest of three values; ties → lowest index. |
| `argmax3_u32` | `Argmax3Wide::run() -> u16` | Index (0, 1, or 2) of the largest of three values at wide u32 width; ties -> lowest index — the wide sibling of argmax3 (which works over u16 and can't rank values beyond 65535, e.g. money totals in cents). |
| `argmin3` | `run(a: u16, b: u16, c: u16) -> u16` | Index (0, 1, or 2) of the smallest of three values; ties → lowest index. |
| `argmin3_u32` | `Argmin3Wide::run() -> u16` | Index (0, 1, or 2) of the smallest of three values at wide u32 width; ties -> lowest index — the wide sibling of argmin3 (which works over u16 and can't rank values beyond 65535, e.g. money totals in cents). |
| `majority3` | `run(a: u16, b: u16, c: u16) -> u16` | Returns 1 if at least two of three values are equal, else 0. |
| `max` | `run(a: u16, b: u16) -> u16` | Maximum of two values. |
| `max3` | `run(a: u16, b: u16, c: u16) -> u16` | Largest of three values. |
| `mean3` | `run(a: u16, b: u16, c: u16) -> u16` | Mean (average) of three values, computed without overflow. |
| `median3` | `run(a: u16, b: u16, c: u16) -> u16` | Median (middle value) of three. |
| `midrange3` | `run(a: u16, b: u16, c: u16) -> u16` | Midrange of three values: (min + max) / 2. |
| `min` | `run(a: u16, b: u16) -> u16` | Minimum of two values. |
| `min3` | `run(a: u16, b: u16, c: u16) -> u16` | Smallest of three values. |
| `mode3` | `run(a: u16, b: u16, c: u16) -> u16` | Mode of three values: the value that repeats (ties/all-distinct → the first, a). |
| `range3` | `run(a: u16, b: u16, c: u16) -> u16` | Spread of three values: max − min. |
| `sum3` | `run(a: u16, b: u16, c: u16) -> u16` | Sum of three values (saturating at 65535). |
| `sum4` | `Sum4::run() -> u16` | Sum of four values (saturating at 65535) — the four-operand sibling of sum3. |

## running-stats (5)

| id | signature | summary |
|---|---|---|
| `accumulate_step` | `Accumulate::run() -> u16` | Running sum + count over a stream of values (sum saturates at 65535). Compose with safe_div(sum, count) for a running mean. |
| `running_min_max_step` | `RunningMinMax::run() -> u16` | Running min/max tracker over a stream of values: updates min/max (self-initializing on the first call via `seen`), returns the current range (max - min). |
| `running_variance_step` | `RunningVariance::run() -> u16` | Running (population) variance over a stream of values, one value per call — the checked/exact sibling of accumulate_step (which saturates u16; this escalates on overflow instead, since a corrupted variance is worse than a stopped one). Recomputes the mean fresh from the exact running sum on each side of the update (rather than compounding a previously-truncated running mean, Welford-style) before accumulating the squared-deviation product into m2 — verified to never go negative under integer truncation across thousands of random and adversarial streams. Compose with div_floor_u32(m2, count) for the variance itself. |
| `streak_step` | `Streak::run() -> u16` | Consecutive-streak counter: increments while input is nonzero, resets to 0 the moment input is 0. |
| `zscore_q8` | `run(value_q8: i16, mean_q8: i16, stddev_q8: i16) -> i16` | Q8.8 fixed-point z-score given an already-computed standard deviation: (value - mean) scaled by 256, divided by stddev — sidesteps the sqrt-of-variance problem cosine_score_approx is still blocked on by taking stddev as an input rather than deriving it. Returns 0 if stddev_q8 <= 0 (the safe_div convention, no divide-by-zero). |

## safe-arith (9)

| id | signature | summary |
|---|---|---|
| `add_sat` | `run(a: u16, b: u16) -> u16` | Saturating add: a + b, capped at 65535 instead of wrapping. |
| `avg2` | `run(a: u16, b: u16) -> u16` | Average of two values, (a + b) / 2, computed without overflow. |
| `ceil_div` | `run(a: u16, b: u16) -> u16` | Ceiling division: the smallest k with k*b >= a (0 if b == 0). Rounds up. |
| `mul_sat` | `run(a: u16, b: u16) -> u16` | Saturating multiply: a * b, capped at 65535 instead of wrapping. |
| `safe_div` | `run(a: u16, b: u16) -> u16` | Integer divide a / b, returning 0 when b == 0 (no divide-by-zero). |
| `safe_mod` | `run(a: u16, b: u16) -> u16` | Remainder a % b, returning 0 when b == 0. |
| `square` | `run(n: u16) -> u16` | Saturating square: n * n, capped at 65535. |
| `square_wide` | `Sq::run() -> u16` | Exact square with a wide u32 result field: sq = n*n, no u16 cap (the value cell square saturates). |
| `sub_sat` | `run(a: u16, b: u16) -> u16` | Saturating subtract: a - b, floored at 0 when b > a. |

## scoring-choice (9)

| id | signature | summary |
|---|---|---|
| `choose_best2` | `ChooseBest2::run() -> u16` | Pick the value of whichever of two (value, score) candidates has the highest score (ties -> lowest index, matching choose_best3's convention) — the 2-candidate sibling of choose_best3, for the common case of only two options (e.g. "which of these two candidates has the highest profit"). |
| `choose_best3` | `ChooseBest3::run() -> u16` | Pick the value of whichever of three (value, score) candidates has the highest score (ties → lowest index, matching argmax3's convention) — distinct from argmax3, which assumes the value and the score are the same number. |
| `choose_worst2` | `ChooseWorst2::run() -> u16` | Pick the value of whichever of two (value, score) candidates has the lowest score (ties -> lowest index, matching choose_best2's convention) — the inverse-comparison sibling of choose_best2, for the common "which of these two costs less" shape. |
| `clear_winner_u32` | `ClearWinnerWide::run() -> u16` | Returns 1 if the top score beats the second-best by at least margin at wide u32 width, else 0 — including when top < second (a malformed call, treated as no clear winner) — the wide sibling of is_clear_winner (which works over u16 and can't compare scores beyond 65535, e.g. money totals in cents). |
| `is_clear_winner` | `run(top: u16, second: u16, margin: u16) -> u16` | Returns 1 if the top score beats the second-best by at least margin (a decisive win, not a near-tie), else 0 — including when top < second (a malformed call, treated as no clear winner). |
| `weighted_sum` | `run(a: u16, b: u16, c: u16) -> u16` | Weighted sum of three inputs with fixed weights 1, 2, 3 (a candidate score). |
| `weighted_sum2` | `WeightedSum2::run() -> u16` | Weighted sum of two inputs with caller-supplied weights: a*wa + b*wb (also known as score_2factor — the same formula under a different name). Sibling of weighted_sum/weighted_sum_wide (which use fixed weights 1, 2, 3), generalized to arbitrary weights, so a genuine u32 overflow is possible and escalates instead of silently wrapping. |
| `weighted_sum3` | `WeightedSum3::run() -> u16` | Weighted sum of three inputs with caller-supplied weights: a*wa + b*wb + c*wc. Sibling of weighted_sum/weighted_sum_wide (fixed weights 1, 2, 3) generalized to arbitrary weights, so a genuine u32 overflow is possible and escalates instead of silently wrapping. |
| `weighted_sum_wide` | `Ws::run() -> u16` | Exact weighted sum with a wide u32 result field: sum = a + 2b + 3c, no u16 wrap (sibling of weighted_sum). |

## sequences (5)

| id | signature | summary |
|---|---|---|
| `arithmetic_nth_u32` | `ArithmeticNthWide::run() -> u16` | The nth term of an arithmetic sequence starting at start with common difference step: start + step*(n-1), 1-indexed (n=1 is the first term) — the missing nth-term sibling of arithmetic_series_sum (which only sums the sequence, not a single term). |
| `arithmetic_series_sum` | `ArithmeticSeriesSum::run() -> u16` | Sum of the first n terms of an arithmetic sequence starting at a with common difference d: n*(2a + (n-1)*d) / 2 — always an exact integer (the product n*(2a+(n-1)*d) is provably always even), checked for overflow at each step. |
| `consecutive_sum_start` | `ConsecutiveSumStart::run() -> u16` | Given n consecutive integers step apart summing to sum, find the first one: first = (sum - step*n*(n-1)/2) / n. Generalizes the "n consecutive integers" and "n consecutive odd/even integers" shapes into one cell via the step parameter (step=1 for consecutive integers, step=2 for consecutive odd/even). Escalates if the split isn't exact or would go negative — a wrong-plan signal. |
| `geometric_nth_checked_u32` | `GeometricNthChecked::run() -> u16` | The nth term of a geometric sequence starting at start with ratio ratio: start * ratio^(n-1), 1-indexed (n=1 is the first term) — the missing nth-term sibling of geometric_series_sum (which only sums the sequence, not a single term). Computed by direct iterative multiplication rather than exponentiation, so it escalates exactly when the true term doesn't fit u32, no earlier. |
| `geometric_series_sum` | `GeometricSeriesSum::run() -> u16` | Sum of the first n terms of a geometric sequence starting at a with ratio r (a + a*r + a*r^2 + ... + a*r^(n-1)), computed by direct iterative summation rather than the a*(r^n-1)/(r-1) closed form — r^n alone would overflow long before a genuinely unrepresentable sum does, so this escalates exactly when the true sum (or an intermediate term) doesn't fit u32, no earlier. Exact for any r >= 0, not just r > 1. |

## signed-deltas (4)

| id | signature | summary |
|---|---|---|
| `abs_i16` | `run(x: i16) -> u16` | Absolute value of a signed 16-bit value, returned as u16 (correctly handles i16::MIN, whose magnitude 32768 doesn't fit back in i16). |
| `apply_delta_clamped` | `run(value: u16, delta: i16, cap: u16) -> u16` | Apply a signed delta to an unsigned value, clamped to [0, cap] — e.g. a health/resource/score adjustment that can't go negative or exceed a cap (a "risk delta" applied safely). |
| `clamp_i16` | `run(x: i16, lo: i16, hi: i16) -> i16` | Clamp a signed value to the inclusive range [lo, hi] — the signed counterpart of clamp (which only works over u16). Also the exact form of "hard tanh" in Q8.8 fixed point (clamp_i16(x, -256, 256)): tanh_hard(x) = 2*sigmoid_hard(2x)-1 reduces algebraically to clamp(x, -1, 1), so q_tanh was deliberately not shipped as a second cell — same formula, different name, exactly the case the admission gate exists to catch. |
| `sign_i16` | `run(x: i16) -> i16` | Sign of a signed value: 1 if positive, -1 if negative, 0 if zero. |

## softfloat (2)

| id | signature | summary |
|---|---|---|
| `lerp_f32` | `LerpF32::run() -> u16` | Linear interpolation a + t*(b - a) in IEEE binary32 — the owned softfloat kernels, correctly rounded per op and bit-identical to rustc f32; t is a plain f32 (not clamped), so t=0 gives a and t=1 gives a + (b - a). |
| `norm2_f32` | `Norm2F32::run() -> u16` | Euclidean length of a 2-vector in IEEE binary32 — sqrt(x*x + y*y) through the owned softfloat kernels: correctly rounded per op, bit-identical to rustc f32, deterministic on every host (no libm). |

## spatial-grid (6)

| id | signature | summary |
|---|---|---|
| `aabb_intersect` | `AabbIntersect::run() -> u16` | Returns 1 if two axis-aligned bounding boxes (x1,y1,w1,h1) and (x2,y2,w2,h2) overlap (edge-touching doesn't count), else 0. |
| `bresenham_step` | `BresenhamStep::run() -> u16` | Bresenham line-drawing, one step: given the fixed line parameters (dx, dy — the absolute deltas between the endpoints) and the running error term (as a sign-magnitude pair, since state fields can't be i16 — err can go negative), reports whether this step advances x, y, or both (step_x/step_y, 0 or 1) and updates the error term. The caller applies step_x/step_y to its own x/y using its own known step directions (sx, sy) — tracking dx/dy/err here and x/y/sx/sy on the caller's side avoids needing four more sign-magnitude field pairs for quantities the error-term math never actually needs to know the sign of. Verified against a full reference line generator across 2,000 random line segments (coordinates up to +/-500) before shipping. |
| `grid_index` | `run(x: u16, y: u16, width: u16) -> u16` | Flat array index of a grid cell (x, y) in a grid of the given row width: y * width + x. |
| `morton_decode` | `MortonDecode::run() -> u16` | Morton (Z-order curve) decode: the inverse of morton_encode — split a u32 spatial index back into its two interleaved u16 coordinates via the same branch-free bit-compaction trick (constant shift amounts, no dynamic-shift loop). |
| `morton_encode` | `MortonEncode::run() -> u16` | Morton (Z-order curve) encode: interleave the bits of two u16 coordinates into one u32 spatial index (x's bits at even positions, y's at odd), so a single integer sorts nearby 2D points near each other — a common spatial-indexing key. The classic branch-free "magic numbers" bit-spread (constant shift amounts, no dynamic-shift loop): needs a u32 state field since interleaving two full u16s produces 32 bits, more than either input's own width. |
| `point_in_rect` | `PointInRect::run() -> u16` | Returns 1 if point (px, py) is inside rect (rx, ry, rw, rh) — half-open: [rx, rx+rw) x [ry, ry+rh) — else 0. |

## stateful-rng (3)

| id | signature | summary |
|---|---|---|
| `counter_step` | `CounterStep::run() -> u16` | Modular counter step: increments count by 1, wrapping to 0 the moment it would reach `limit` (limit 0 means never wrap — a plain saturating-free incrementer). Useful for round-robin dispatch or a bounded retry index. The caller threads `count` through — re-supply the field each call. |
| `lcg_next` | `Lcg::run() -> u16` | Linear congruential generator step: seed = seed * 1664525 + 1013904223 (mod 2^32, Numerical Recipes constants), returning the top 16 bits (the higher bits of an LCG are far less patterned than the low bits). The caller threads `seed` through — re-supply the field each call, since state cells don't persist memory across separate runs. |
| `xorshift16` | `Xorshift16::run() -> u16` | 16-bit xorshift generator step (x ^= x<<7; x ^= x>>9; x ^= x<<8) — a distinct pseudo-random recurrence from lcg_next (no multiply, pure shift/xor). The caller threads `x` through — re-supply the field each call. A seed of 0 is a fixed point (0 forever); always seed nonzero. |

## statistics (2)

| id | signature | summary |
|---|---|---|
| `covariance` | `Covariance::run() -> u16` | Population covariance from precomputed sums (not a raw dataset -- that aggregation stays upstream, matching running_variance_step's own bivariate framing): cov = (n*sum_xy - sum_x*sum_y) / n^2, returned as an exact signed fraction (num/den, den always positive) rather than rounded to an integer. |
| `linear_regression_slope` | `LinearRegressionSlope::run() -> u16` | Ordinary-least-squares regression slope from precomputed sums (n, sum_x, sum_y, sum_xy, sum_x2 -- raw-dataset aggregation stays upstream): slope = (n*sum_xy - sum_x*sum_y) / (n*sum_x2 - sum_x^2), returned as an exact signed fraction rather than rounded. The denominator is n^2 times the population variance of x, which is always non-negative by construction -- zero only when every x value is identical (a vertical "line", no defined slope). |

## units (4)

| id | signature | summary |
|---|---|---|
| `same_unit_check` | `run(a: u16, b: u16) -> u16` | Unit-compatibility check for adding/subtracting two typed quantities: returns their shared dimension code if the units match, else escalates on a units mismatch (dimension codes documented in docs/library-growth.md, now including 8=rate_money_per_time and 9=rate_count_per_time). |
| `unit_cancel_check` | `run(a: u16, b: u16) -> u16` | Returns 1 if dividing a numerator-unit quantity by a denominator-unit quantity is dimensionally defined (same rule table as unit_div), else 0 — a non-escalating probe for a caller (e.g. a plan verifier) trying several candidate unit pairs without halting. |
| `unit_div` | `run(a: u16, b: u16) -> u16` | Resulting unit-dimension code when dividing a numerator quantity by a denominator quantity (e.g. money/count=rate_money_per_count, money/time=rate_money_per_time, count/time=rate_count_per_time, same/same=count) — same codes as unit_mul (docs/library-growth.md). Escalates on any unmodeled pair. |
| `unit_mul` | `run(a: u16, b: u16) -> u16` | Resulting unit-dimension code when multiplying two typed quantities (e.g. count*money=money, distance*distance=area) — codes: 0=count,1=money,2=time,3=distance,4=area,5=volume,6=rate_money_per_count,7=rate_distance_per_time,8=rate_money_per_time,9=rate_count_per_time (docs/library-growth.md). Escalates on any unmodeled pair. |

## validation (1)

| id | signature | summary |
|---|---|---|
| `range_check` | `run(x: u16, lo: u16, hi: u16) -> u16` | Returns 1 if lo <= x <= hi, else 0. |

## vector (6)

| id | signature | summary |
|---|---|---|
| `cross_product` | `CrossProduct::run() -> u16` | Cross product of two 3D vectors: (ay*bz - az*by, az*bx - ax*bz, ax*by - ay*bx). Each signed component is tracked as a (magnitude, sign) pair through the multiply and the combining subtract -- the same technique vectors_parallel uses for its equality checks, extended one step further here since a real signed result (not just a zero/nonzero check) is needed. The result can exceed either input's own magnitude, so it rides wide u32-magnitude output fields rather than being narrowed back to i16. |
| `dot2` | `Dot2::run() -> u16` | Dot product of two 2D vectors (ax, ay) and (bx, by): ax*bx + ay*by. |
| `norm2_sq` | `run(x: u16, y: u16) -> u16` | Squared magnitude of a 2D vector (x, y): x*x + y*y (no sqrt). |
| `triple_scalar_product` | `TripleScalarProduct::run() -> u16` | Triple scalar product a . (b x c) of three 3D vectors -- the signed volume of the parallelepiped they span (zero exactly when the three vectors are coplanar). Computed as cross_product(b, c) followed by a signed dot with a, reusing the same (magnitude, sign) tracking cross_product and vectors_parallel already establish, so no new arithmetic technique is introduced here. |
| `triple_vector_product` | `TripleVectorProduct::run() -> u16` | Triple vector product a x (b x c) of three 3D vectors, via the BAC-CAB identity a x (b x c) = b*(a.c) - c*(a.b) -- pure dot-products and scalar multiplies, never an actual cross-product computation. Each stage (the two dot products, the two vector scalings, the final vector subtract) is tracked as a (magnitude, sign) pair, the same discipline cross_product/triple_scalar_product use. Genuinely narrower-range than those two: scaling a vector component by a dot product can reach i16's product-of-three-factors territory, so this escalates well before either input vector's own magnitude would suggest trouble. |
| `vectors_parallel` | `VectorsParallel::run() -> u16` | Check whether two 3D vectors are parallel (or anti-parallel) -- one is a scalar multiple of the other. Computed via three pairwise-product equality checks (same magnitude, same sign, or either magnitude zero) rather than a signed subtract, so no sign-combining step is needed at all. |

## verifier-ranker (19)

| id | signature | summary |
|---|---|---|
| `agree3_u32` | `Agree3Wide::run() -> u16` | Multi-plan agreement check at wide u32 width: returns 1 if at least two of three candidate answers are equal, else 0 — the wide sibling of majority3 (which works over u16 and can't represent answers beyond 65535, e.g. money totals in cents). |
| `answer_eq_u32` | `AnswerEqWide::run() -> u16` | Verifies a claimed wide answer: returns 1 if a == b, else 0 — the wide sibling of eq (which works over u16 and can't compare values beyond 65535, e.g. money totals in cents). |
| `answer_within_tolerance_u32` | `AnswerWithinToleranceWide::run() -> u16` | Verifies a claimed wide answer is within an absolute tolerance of the true value: returns 1 if \|candidate - actual\| <= tolerance, else 0 — distinct from within_percent (a percentage-based tolerance over u16); this is an absolute margin at wide u32 width. |
| `diff_equals` | `run(a: u16, b: u16, remainder: u16) -> u16` | Verifies a claimed difference: returns 1 if a >= b and a - b == remainder, else 0 (including when a < b, since an unsigned difference can't be negative). |
| `diff_equals_u32` | `DiffEqualsWide::run() -> u16` | Verifies a claimed wide difference: returns 1 if a >= b and a - b == remainder, else 0 (including when a < b, since an unsigned difference can't be negative) — the wide sibling of diff_equals (which works over u16). |
| `mul_add_equals_u32` | `MulAddEqualsWide::run() -> u16` | Verifies a claimed wide fused multiply-add: returns 1 if a * b + c == total, else 0, including when either step overflows u32 — the reverse-equation counterpart of mul_add_checked_u32. |
| `mul_sub_equals_u32` | `MulSubEqualsWide::run() -> u16` | Verifies a claimed wide fused multiply-subtract: returns 1 if a * b - c == total, else 0, including when the multiply overflows u32 or c exceeds the product — the reverse-equation counterpart of mul_sub_checked_u32. |
| `nonnegative_after_delta` | `run(value: u16, delta: i16) -> u16` | Returns 1 if applying a signed delta to an unsigned value would stay nonnegative, else 0 — the boolean-verdict form of the sign-handling idiom apply_delta_clamped already uses, for a caller (e.g. a plan verifier) that wants to kill a wrong "subtract too much" plan cheaply without needing the clamped value itself. |
| `parts_sum_to_total4_u32` | `PartsSumToTotal4Wide::run() -> u16` | Verifies a claimed wide four-way sum: returns 1 if a + b + c + d == total, else 0, without escalating on overflow — the missing four-way sibling of sum3_equals_u32 (a real gap: every prior verifier-ranker sum shape topped out at three parts). |
| `percent_equals_bps` | `PercentEqualsBps::run() -> u16` | Verifies a claimed bps increase: returns 1 if after == before + before*bps/10000, else 0 — the verifier counterpart of increase_by_bps (money-bps's checked-arithmetic sibling had no reverse-equation check yet, unlike every other checked-arithmetic shape). Never escalates: a verifier always returns a verdict, computed in a wider internal width so a genuine overflow can't false-positive as a match. |
| `pow_equals_u32` | `PowEqualsWide::run() -> u16` | Verifies a claimed wide power: returns 1 if base^exp == total, else 0, including when an intermediate multiply overflows u32 — the reverse-equation counterpart of pow_checked_u32. |
| `product3_equals_u32` | `Product3EqualsWide::run() -> u16` | Verifies a claimed wide three-way product: returns 1 if a * b * c == total, else 0, including when the product overflows u32 (a real overflow just means the claim doesn't hold) — the reverse-equation counterpart of mul3_checked_u32. |
| `product_equals_u32` | `ProductEquals::run() -> u16` | Verifies a claimed wide product: 1 if a * b == total, else 0 — including when a * b overflows u32 (a real overflow just means the claim doesn't hold, not an escalation; a verifier always returns a verdict). |
| `quotient_equals_exact_u32` | `QuotientEqualsExact::run() -> u16` | Verifies a claimed exact wide quotient: 1 if b != 0, a divides evenly by b (a % b == 0), and a / b == quotient, else 0 — the verifier counterpart of div_exact_u32 (that one computes and escalates on a remainder; this one checks a candidate answer and always returns a verdict). |
| `smag_eq` | `SmagEq::run() -> u16` | Verifies whether two signed values (magnitude, sign pairs, per smag_add) are equal, canonicalizing negative-zero to nonnegative first — the sign-magnitude counterpart of frac_eq / answer_eq_u32. |
| `smag_is_nonneg` | `SmagIsNonneg::run() -> u16` | Constraint check for a signed-magnitude quantity (magnitude, sign pair — neg 0=nonnegative, 1=negative, per smag_add): returns 1 if the value is nonnegative (neg == 0, or magnitude == 0 regardless of the sign flag), else 0. |
| `sum3_equals_u32` | `Sum3EqualsWide::run() -> u16` | Verifies a claimed wide three-way sum: returns 1 if a + b + c == total, else 0, without escalating on overflow — the reverse-equation counterpart of add3_checked_u32. |
| `sum_equals` | `run(a: u16, b: u16, total: u16) -> u16` | Verifies a claimed sum: returns 1 if a + b == total, else 0 — computed in a wider internal width so a genuine overflow can't false-positive as a match on the wrapped value. |
| `sum_equals_u32` | `SumEqualsWide::run() -> u16` | Verifies a claimed wide sum: returns 1 if a + b == total, else 0, without escalating on overflow (a real overflow just means the claim doesn't hold) — the wide sibling of sum_equals (which works over u16). |

## aliases (4)

Behaviourally identical to a landed cell (found by the Phase 2.2 admission gate); removed as separate code, vocabulary merged into the surviving cell's tags.

| old id | → | landed as |
|---|---|---|
| `argmin2` | → | `is_gt` |
| `argmax2` | → | `is_lt` |
| `quantize` | → | `safe_div` |
| `wrap` | → | `safe_mod` |

## planned (not yet landed)

See `docs/library-growth.md` "Next waves" for the prioritized list (stateful/RNG, signed-deltas, and scoring/choice's second slice above are landed — bounded_rand and time/budget's five named candidates were all found to be exact duplicates of existing cells, not built; score_2factor's vocabulary was merged into weighted_sum2's tags rather than shipping a duplicate; cosine_score_approx still ahead), the Phase 2.3 pilot-batch section for the author->verify->admit loop, and `docs/math-campaign-spec.md` for the GSM8K math campaign (M1 complete: checked-arithmetic, money-bps, units, verifier-ranker, and fractions above are all five authored packs — M0 landed Tier 2, one u32 param per call, so fractions inlines its own GCD-reduction loop per cell rather than sharing a two-u32-param gcd_u32 helper; M2-M4 remain gated behind cell_solve). All five originally-planned wave-3 packs plus the Phase 2.3 pilot batch (packing/BCD, vector) landed a first slice above. `unpack_lo`/`unpack_hi` were never built — checking docs/cell-index.md before authoring found they'd be exact duplicates of `low_byte`/`high_byte`. Each first slice deferred its harder items: ISBN/IBAN/UPC checksums need a wider-than-u32 input (see library-growth.md); q_sqrt/piecewise sigmoid-tanh, rate_window_update, a fixed-point running variance (Welford), Morton encode/decode (needs a u32 state field, not yet risked), a Bresenham stepper, and cosine_score_approx (deferred: exact fixed-point cosine needs a wide sqrt-of-a-product without overflow, not yet worked out) are all still open.
