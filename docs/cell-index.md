# Cell index — every landed cell, by pack

*Generated from `cell80/cells` (239 cells) by `cell80/scripts/gen_cell_index.py`. Regenerate after any cell is added/removed:*

```
cargo run -q -p cell80 --bin cell80 -- index cell80/cells --json \
  | python3 cell80/scripts/gen_cell_index.py > docs/cell-index.md
```

See `docs/library-growth.md` for the packs' purpose, the contribution rule, and the admission gate that enforces "no behavioural duplicates."

## predicates (10)

| id | signature | summary |
|---|---|---|
| `eq` | `run(a: u16, b: u16) -> u16` | Returns 1 if a == b, else 0. |
| `neq` | `run(a: u16, b: u16) -> u16` | Returns 1 if a != b, else 0. |
| `is_lt` | `run(a: u16, b: u16) -> u16` | Returns 1 if a < b (strictly less than), else 0. |
| `is_le` | `run(a: u16, b: u16) -> u16` | Returns 1 if a <= b (at most), else 0. |
| `is_gt` | `run(a: u16, b: u16) -> u16` | Returns 1 if a > b (strictly greater than), else 0. |
| `is_ge` | `run(a: u16, b: u16) -> u16` | Returns 1 if a >= b (at least), else 0. |
| `is_zero` | `run(x: u16) -> u16` | Returns 1 if x is zero, else 0. |
| `nonzero` | `run(x: u16) -> u16` | Returns 1 if x is nonzero, else 0. |
| `is_even` | `run(x: u16) -> u16` | Returns 1 if x is even, else 0. |
| `is_odd` | `run(x: u16) -> u16` | Returns 1 if x is odd, else 0. |

## safe-arith (9)

| id | signature | summary |
|---|---|---|
| `add_sat` | `run(a: u16, b: u16) -> u16` | Saturating add: a + b, capped at 65535 instead of wrapping. |
| `sub_sat` | `run(a: u16, b: u16) -> u16` | Saturating subtract: a - b, floored at 0 when b > a. |
| `mul_sat` | `run(a: u16, b: u16) -> u16` | Saturating multiply: a * b, capped at 65535 instead of wrapping. |
| `safe_div` | `run(a: u16, b: u16) -> u16` | Integer divide a / b, returning 0 when b == 0 (no divide-by-zero). |
| `safe_mod` | `run(a: u16, b: u16) -> u16` | Remainder a % b, returning 0 when b == 0. |
| `ceil_div` | `run(a: u16, b: u16) -> u16` | Ceiling division: the smallest k with k*b >= a (0 if b == 0). Rounds up. |
| `avg2` | `run(a: u16, b: u16) -> u16` | Average of two values, (a + b) / 2, computed without overflow. |
| `square` | `run(n: u16) -> u16` | Saturating square: n * n, capped at 65535. |
| `square_wide` | `Sq::run() -> u16` | Exact square with a wide u32 result field: sq = n*n, no u16 cap (the value cell square saturates). |

## bounds (6)

| id | signature | summary |
|---|---|---|
| `between_exclusive` | `run(x: u16, lo: u16, hi: u16) -> u16` | Returns 1 if lo < x < hi (strictly inside, exclusive bounds), else 0. |
| `normalize_0_100` | `run(x: u16, lo: u16, hi: u16) -> u16` | Rescale x within [lo, hi] to a 0..100 percentage (clamped; 0 if hi <= lo). |
| `snap_down` | `run(x: u16, step: u16) -> u16` | Round x DOWN to the nearest multiple of step (x if step == 0). Floor to grid. |
| `snap_up` | `run(x: u16, step: u16) -> u16` | Round x UP to the nearest multiple of step (x if step == 0). Ceil to grid. |
| `round_to_multiple` | `run(x: u16, step: u16) -> u16` | Round x to the NEAREST multiple of step (ties up; x if step == 0). |
| `clamp` | `run(x: u16, lo: u16, hi: u16) -> u16` | Clamp a value to the inclusive range [lo, hi]. |

## validation (1)

| id | signature | summary |
|---|---|---|
| `range_check` | `run(x: u16, lo: u16, hi: u16) -> u16` | Returns 1 if lo <= x <= hi, else 0. |

## percent (7)

| id | signature | summary |
|---|---|---|
| `percent` | `run(part: u16, whole: u16) -> u16` | Percentage of a whole: part*100/whole, in 0..100+ (0 if whole == 0). |
| `permille` | `run(part: u16, whole: u16) -> u16` | Per-mille (parts per thousand): part*1000/whole (0 if whole == 0). |
| `ratio_255` | `run(part: u16, whole: u16) -> u16` | Ratio scaled to a 0..255 byte fraction: part*255/whole (0 if whole == 0). |
| `scale_percent` | `run(value: u16, pct: u16) -> u16` | Take pct percent of a value: value*pct/100. |
| `increase_percent` | `run(value: u16, pct: u16) -> u16` | Increase a value by pct percent: value + value*pct/100 (saturating at 65535). |
| `discount_percent` | `run(value: u16, pct: u16) -> u16` | Decrease a value by pct percent: value - value*pct/100 (0 if pct >= 100). |
| `within_percent` | `run(actual: u16, target: u16, pct: u16) -> u16` | Returns 1 if actual is within pct percent of target (\|actual-target\|*100 <= target*pct). |

## ranking-stats (13)

| id | signature | summary |
|---|---|---|
| `min` | `run(a: u16, b: u16) -> u16` | Minimum of two values. |
| `max` | `run(a: u16, b: u16) -> u16` | Maximum of two values. |
| `min3` | `run(a: u16, b: u16, c: u16) -> u16` | Smallest of three values. |
| `max3` | `run(a: u16, b: u16, c: u16) -> u16` | Largest of three values. |
| `median3` | `run(a: u16, b: u16, c: u16) -> u16` | Median (middle value) of three. |
| `argmax3` | `run(a: u16, b: u16, c: u16) -> u16` | Index (0, 1, or 2) of the largest of three values; ties → lowest index. |
| `argmin3` | `run(a: u16, b: u16, c: u16) -> u16` | Index (0, 1, or 2) of the smallest of three values; ties → lowest index. |
| `sum3` | `run(a: u16, b: u16, c: u16) -> u16` | Sum of three values (saturating at 65535). |
| `mean3` | `run(a: u16, b: u16, c: u16) -> u16` | Mean (average) of three values, computed without overflow. |
| `range3` | `run(a: u16, b: u16, c: u16) -> u16` | Spread of three values: max − min. |
| `mode3` | `run(a: u16, b: u16, c: u16) -> u16` | Mode of three values: the value that repeats (ties/all-distinct → the first, a). |
| `majority3` | `run(a: u16, b: u16, c: u16) -> u16` | Returns 1 if at least two of three values are equal, else 0. |
| `midrange3` | `run(a: u16, b: u16, c: u16) -> u16` | Midrange of three values: (min + max) / 2. |

## bit/mask (11)

| id | signature | summary |
|---|---|---|
| `popcount` | `run(x: u16) -> u16` | Population count: the number of set bits in a 16-bit value. |
| `parity` | `run(x: u16) -> u16` | Parity: 1 if the number of set bits is odd, else 0. |
| `bit_is_set` | `run(x: u16, bit: u16) -> u16` | Returns 1 if bit number `bit` of x is set, else 0. |
| `set_bit` | `run(x: u16, bit: u16) -> u16` | Set bit number `bit` of x to 1. |
| `clear_bit` | `run(x: u16, bit: u16) -> u16` | Clear bit number `bit` of x to 0. |
| `toggle_bit` | `run(x: u16, bit: u16) -> u16` | Toggle (flip) bit number `bit` of x. |
| `mask_has_all` | `run(x: u16, mask: u16) -> u16` | Returns 1 if x has ALL bits of mask set: (x & mask) == mask. |
| `mask_has_any` | `run(x: u16, mask: u16) -> u16` | Returns 1 if x has ANY bit of mask set: (x & mask) != 0. |
| `mask_union` | `run(a: u16, b: u16) -> u16` | Union of two bit masks: a \| b (every bit set in either). |
| `mask_intersection` | `run(a: u16, b: u16) -> u16` | Intersection of two bit masks: a & b (bits set in both). |
| `mask_xor` | `run(a: u16, b: u16) -> u16` | Symmetric difference of two bit masks: a ^ b (bits set in exactly one). |

## number-theory (30)

| id | signature | summary |
|---|---|---|
| `lcm` | `run(a: u16, b: u16) -> u16` | Least common multiple of two values (a/gcd*b; 0 if either is 0). u16 domain. |
| `gcd` | `run(a: u16, b: u16) -> u16` | Greatest common divisor (Euclid's algorithm). |
| `gcd3` | `run(a: u16, b: u16, c: u16) -> u16` | Greatest common divisor of three values. |
| `lcm3` | `run(a: u16, b: u16, c: u16) -> u16` | Least common multiple of three values. |
| `divides` | `run(a: u16, b: u16) -> u16` | Returns 1 if a divides b evenly (b % a == 0, a != 0), else 0. |
| `is_coprime` | `run(a: u16, b: u16) -> u16` | Returns 1 if a and b are coprime (gcd == 1), else 0. |
| `is_prime` | `run(n: u16) -> u16` | Returns 1 if n is prime, else 0. |
| `is_square` | `run(n: u16) -> u16` | Returns 1 if n is a perfect square, else 0. |
| `isqrt` | `run(n: u16) -> u16` | Integer square root: the largest r with r*r <= n. |
| `digit_sum` | `run(n: u16) -> u16` | Sum of the decimal digits of n. |
| `num_digits` | `run(n: u16) -> u16` | Number of decimal digits of n (0 has 1 digit). |
| `factor_count` | `run(n: u16) -> u16` | Number of positive divisors of n (0 for n == 0). |
| `triangular` | `run(n: u16) -> u16` | nth triangular number: 1+2+...+n = n*(n+1)/2 (overflow-safe; u16 domain n <= 361). |
| `next_pow2` | `run(n: u16) -> u16` | Smallest power of two >= n (0 if it would exceed 65535; next_pow2(0) = 1). |
| `is_pow2` | `run(x: u16) -> u16` | Returns 1 if x is a power of two, else 0. |
| `pow_small` | `run(base: u16, exp: u16) -> u16` | base raised to exp (saturating at 65535). 0^0 = 1. |
| `cube_sat` | `run(n: u16) -> u16` | Saturating cube: n*n*n, capped at 65535 (n >= 41 saturates). |
| `pow_mod` | `run(base: u16, exp: u16, m: u16) -> u16` | Modular exponentiation: (base^exp) mod m (0 if m == 0). u16 domain m <= 256. |
| `pow_mod_u32` | `PowModWide::run() -> u16` | Modular exponentiation at wide u32 width: (base^exp) mod m — the wide sibling of pow_mod (u16 domain, m <= 256); lifts the modulus ceiling to 65536, wide enough for AIME's "find the remainder mod 1000" finishing move. Returns 0 if m == 0, matching pow_mod's convention. |
| `mod_add_u32` | `ModAddWide::run() -> u16` | Modular addition at wide u32 width: (a + b) mod m — reduces both operands mod m first, so a and b need not already be canonical residues. |
| `mod_sub_u32` | `ModSubWide::run() -> u16` | Modular subtraction at wide u32 width: (a - b) mod m, always returned in [0, m) — e.g. 3 - 5 mod 7 = 5, not a negative remainder. |
| `mod_mul_u32` | `ModMulWide::run() -> u16` | Modular multiplication at wide u32 width: (a * b) mod m — reduces both operands mod m first, then multiplies; the non-exponentiating sibling of pow_mod_u32, sharing its overflow bound. |
| `sum_divisors` | `SumDivisors::run() -> u16` | Sum of the positive divisors of n (n >= 1), including 1 and n itself (sigma(n)) — the sum-valued sibling of factor_count (which counts divisors; this sums them, so it needs a wide result field since sigma(n) routinely exceeds 65535 within the u16 domain). |
| `euler_totient` | `run(n: u16) -> u16` | Euler's totient (phi): count of integers in [1, n] coprime to n (n >= 1; phi(1) = 1 by convention). |
| `smallest_prime_factor` | `run(n: u16) -> u16` | Smallest prime factor of n (n >= 2) — the least prime p dividing n; returns n itself if n is prime. |
| `digit_reverse` | `run(n: u16) -> u16` | Reverse the decimal digits of n (e.g. 123 -> 321; trailing zeros drop, so 120 -> 21). |
| `digit_product` | `run(n: u16) -> u16` | Product of the decimal digits of n (0 has product 0, its only digit). |
| `is_prime_u32` | `IsPrimeWide::run() -> u16` | Returns 1 if n is prime at wide u32 width, else 0 — the wide sibling of is_prime (which works over u16, up to 65535). Trial division scales with sqrt(n): a large prime near u32::MAX needs on the order of tens of millions of cycles, far past the 2,000,000 default — pass a larger --cycles budget explicitly for n much beyond a few million. |
| `mod_inverse` | `ModInverse::run() -> u16` | Modular multiplicative inverse of a mod m: the x in [0, m) with a*x == 1 (mod m), via the iterative extended Euclidean algorithm. The Bezout coefficient tracked along the way can go negative, so it's carried as a sign-magnitude pair inline (no shared smag_* subroutine call — a u32 value still can't cross more than one call boundary), the same convention smag_add/pow_mod_u32 use. |
| `crt_solve_pair` | `CrtSolvePair::run() -> u16` | Chinese Remainder Theorem for two congruences: the unique x in [0, m1*m2) with x == r1 (mod m1) and x == r2 (mod m2), when m1 and m2 are coprime. Computes the inverse of m1 modulo m2 via an inlined extended Euclidean algorithm (the same one mod_inverse uses — duplicated here rather than called, since a u32 value still can't cross more than one call boundary), then combines it with the standard closed-form x = r1 + m1*((r2-r1)*inv(m1, m2) mod m2). |

## distance (4)

| id | signature | summary |
|---|---|---|
| `abs_diff` | `run(a: u16, b: u16) -> u16` | Absolute difference \|a - b\| between two values. |
| `manhattan` | `Pts::run() -> u16` | Manhattan distance between two grid points (typed state). |
| `chebyshev` | `Pts::run() -> u16` | Chebyshev (chessboard) distance between two grid points: max(\|dx\|, \|dy\|). |
| `euclid_sq` | `Pts::run() -> u16` | Squared Euclidean distance between two grid points: dx*dx + dy*dy (no sqrt). Wide u32 dist field. |

## bit-encoding (9)

| id | signature | summary |
|---|---|---|
| `low_byte` | `run(x: u16) -> u16` | Low byte of x (x & 0xFF). |
| `high_byte` | `run(x: u16) -> u16` | High byte of x (x >> 8). |
| `swap_bytes` | `run(x: u16) -> u16` | Swap the high and low bytes of x ((x << 8) \| (x >> 8)). |
| `rotl16` | `run(x: u16, n: u16) -> u16` | Rotate the 16 bits of x left by n (n taken mod 16). |
| `rotr16` | `run(x: u16, n: u16) -> u16` | Rotate the 16 bits of x right by n (n taken mod 16). |
| `reverse_bits` | `run(x: u16) -> u16` | Reverse the 16 bits of x (bit 0 <-> bit 15, ...). |
| `leading_zeros` | `run(x: u16) -> u16` | Count of leading (high) zero bits in the 16-bit value (16 for x == 0). |
| `trailing_zeros` | `run(x: u16) -> u16` | Count of trailing (low) zero bits in the 16-bit value (16 for x == 0). |
| `bit_length` | `run(x: u16) -> u16` | Number of bits needed to represent x: index of the highest set bit + 1 (0 for x == 0). |

## hashing (4)

| id | signature | summary |
|---|---|---|
| `hash_pair` | `run(a: u16, b: u16) -> u16` | Deterministic hash mixing two values into one u16. |
| `fnv1a_step` | `run(hash: u16, byte: u16) -> u16` | One FNV-1a-style hash step over a byte: (hash ^ byte) * prime (16-bit). |
| `crc8_step` | `run(crc: u16, byte: u16) -> u16` | One CRC-8 (Dallas/Maxim, poly 0x8C reflected) step over a byte. |
| `mix16` | `run(x: u16) -> u16` | Avalanche-mix one u16 into a well-scrambled u16 (a finalizer / hash of one value). |

## bucket/convert (3)

| id | signature | summary |
|---|---|---|
| `bucket3` | `run(x: u16, t1: u16, t2: u16) -> u16` | Bucket x into 0, 1, or 2 by two ascending thresholds: x<t1 → 0, x<t2 → 1, else 2. |
| `percent_to_byte` | `run(p: u16) -> u16` | Convert a 0..100 percent to a 0..255 byte scale: p*255/100. |
| `byte_to_percent` | `run(b: u16) -> u16` | Convert a 0..255 byte scale to a 0..100 percent: b*100/255. |

## scoring/choice (6)

| id | signature | summary |
|---|---|---|
| `weighted_sum` | `run(a: u16, b: u16, c: u16) -> u16` | Weighted sum of three inputs with fixed weights 1, 2, 3 (a candidate score). |
| `weighted_sum_wide` | `Ws::run() -> u16` | Exact weighted sum with a wide u32 result field: sum = a + 2b + 3c, no u16 wrap (sibling of weighted_sum). |
| `weighted_sum2` | `WeightedSum2::run() -> u16` | Weighted sum of two inputs with caller-supplied weights: a*wa + b*wb (also known as score_2factor — the same formula under a different name). Sibling of weighted_sum/weighted_sum_wide (which use fixed weights 1, 2, 3), generalized to arbitrary weights, so a genuine u32 overflow is possible and escalates instead of silently wrapping. |
| `weighted_sum3` | `WeightedSum3::run() -> u16` | Weighted sum of three inputs with caller-supplied weights: a*wa + b*wb + c*wc. Sibling of weighted_sum/weighted_sum_wide (fixed weights 1, 2, 3) generalized to arbitrary weights, so a genuine u32 overflow is possible and escalates instead of silently wrapping. |
| `choose_best3` | `ChooseBest3::run() -> u16` | Pick the value of whichever of three (value, score) candidates has the highest score (ties → lowest index, matching argmax3's convention) — distinct from argmax3, which assumes the value and the score are the same number. |
| `is_clear_winner` | `run(top: u16, second: u16, margin: u16) -> u16` | Returns 1 if the top score beats the second-best by at least margin (a decisive win, not a near-tie), else 0 — including when top < second (a malformed call, treated as no clear winner). |

## calendrical/checksum (4)

| id | signature | summary |
|---|---|---|
| `is_leap_year` | `run(year: u16) -> u16` | Returns 1 if year is a Gregorian leap year, else 0: divisible by 4, except centuries not divisible by 400. |
| `days_in_month` | `run(month: u16, is_leap: u16) -> u16` | Number of days in a month (1-12; 0 for an invalid month), given a leap-year flag for February. |
| `day_of_week` | `run(year: u16, month: u16, day: u16) -> u16` | Day of week for a Gregorian date via Zeller's congruence: 0=Saturday, 1=Sunday, 2=Monday, ... 6=Friday. |
| `luhn_check` | `run(n: u16) -> u16` | Returns 1 if n's decimal digits pass the Luhn checksum (mod 10, doubling every second digit from the right), else 0. |

## fixed-point (5)

| id | signature | summary |
|---|---|---|
| `q_mul` | `run(a: u16, b: u16) -> u16` | Q8.8 fixed-point multiply: (a * b) >> 8, computed wide so the 16.16 intermediate doesn't overflow. |
| `q_div` | `run(a: u16, b: u16) -> u16` | Q8.8 fixed-point divide: (a << 8) / b, returning 0 when b == 0 (no divide-by-zero). |
| `q_lerp` | `run(a: u16, b: u16, t: u16) -> u16` | Linear interpolation from a to b by t (Q0.8 fraction, 0..256 = 0.0..1.0): a + (b-a)*t/256. Also an EMA step: q_lerp(prev, sample, alpha). |
| `q_sqrt` | `run(x: u16) -> u16` | Q8.8 fixed-point square root: sqrt(x/256)*256, via a branch-free bitwise integer square root on the widened x*256 (u32 only as a local, never a call param/return — the pattern every Q8.8 free function follows). A naive linear-scan integer sqrt was tried first and cost 3.6M cycles at the domain extreme (past the 2,000,000 default); this bitwise version costs under 20,000. |
| `q_sigmoid` | `run(x: i16) -> u16` | Q8.8 fixed-point "hard sigmoid": a well-known piecewise-linear stand-in for the true sigmoid, clamp(x/4 + 0.5, 0, 1) — exact at x=0, saturating to 0/1 outside roughly [-4, 4], monotonic and cheap everywhere between. Input is signed (Q8.8, negative values meaningful, e.g. -256 = -1.0); output is unsigned Q8.8 in [0, 256] (0.0 to 1.0). q_tanh is deliberately not a separate cell: the same derivation (tanh(x) = 2*sigmoid(2x)-1) reduces to clamp_i16(x, -256, 256) exactly, already covered by that cell's own tags. |

## agentic-runtime (6)

| id | signature | summary |
|---|---|---|
| `token_bucket_step` | `TokenBucket::run() -> u16` | Token-bucket rate limiter step: refill by `refill`, cap at `capacity`, then try to spend `cost`; 1 if allowed, 0 if not enough tokens (tokens still refill either way). |
| `backoff_next` | `Backoff::run() -> u16` | Capped exponential backoff: next = min(current * 2, cap), starting at 1 when current is 0. |
| `circuit_breaker_step` | `CircuitBreaker::run() -> u16` | Circuit-breaker state machine step: closed(0) counts failures and opens at the threshold; open(1) waits for cooldown then tries half-open(2); half-open resolves to closed on success or back to open on failure. |
| `debounce_step` | `Debounce::run() -> u16` | Debounce a noisy 0/1 signal: only confirms a change to `input` once it's held for `threshold` consecutive steps; output is the last confirmed-stable value. |
| `hysteresis` | `Hysteresis::run() -> u16` | Hysteresis (Schmitt-trigger) state: turns on at value >= high, turns off at value <= low, else holds the prior state (the dead zone between them). |
| `rate_window_update` | `RateWindowUpdate::run() -> u16` | Fixed-window rate limiter step: given the current time `now`, the running window's start and size, and the count so far, rolls over to a fresh window (starting at `now`) once `now - window_start >= window_size`, then allows the event if `count < limit` (incrementing count) — distinct from token_bucket_step's smooth refill-and-spend model, this is the simpler "N events per window" shape. The caller threads window_start/count through repeated calls, matching backoff_next/token_bucket_step's convention. |

## running-stats (4)

| id | signature | summary |
|---|---|---|
| `running_min_max_step` | `RunningMinMax::run() -> u16` | Running min/max tracker over a stream of values: updates min/max (self-initializing on the first call via `seen`), returns the current range (max - min). |
| `streak_step` | `Streak::run() -> u16` | Consecutive-streak counter: increments while input is nonzero, resets to 0 the moment input is 0. |
| `accumulate_step` | `Accumulate::run() -> u16` | Running sum + count over a stream of values (sum saturates at 65535). Compose with safe_div(sum, count) for a running mean. |
| `running_variance_step` | `RunningVariance::run() -> u16` | Running (population) variance over a stream of values, one value per call — the checked/exact sibling of accumulate_step (which saturates u16; this escalates on overflow instead, since a corrupted variance is worse than a stopped one). Recomputes the mean fresh from the exact running sum on each side of the update (rather than compounding a previously-truncated running mean, Welford-style) before accumulating the squared-deviation product into m2 — verified to never go negative under integer truncation across thousands of random and adversarial streams. Compose with div_floor_u32(m2, count) for the variance itself. |

## spatial/grid (6)

| id | signature | summary |
|---|---|---|
| `grid_index` | `run(x: u16, y: u16, width: u16) -> u16` | Flat array index of a grid cell (x, y) in a grid of the given row width: y * width + x. |
| `point_in_rect` | `PointInRect::run() -> u16` | Returns 1 if point (px, py) is inside rect (rx, ry, rw, rh) — half-open: [rx, rx+rw) x [ry, ry+rh) — else 0. |
| `aabb_intersect` | `AabbIntersect::run() -> u16` | Returns 1 if two axis-aligned bounding boxes (x1,y1,w1,h1) and (x2,y2,w2,h2) overlap (edge-touching doesn't count), else 0. |
| `morton_encode` | `MortonEncode::run() -> u16` | Morton (Z-order curve) encode: interleave the bits of two u16 coordinates into one u32 spatial index (x's bits at even positions, y's at odd), so a single integer sorts nearby 2D points near each other — a common spatial-indexing key. The classic branch-free "magic numbers" bit-spread (constant shift amounts, no dynamic-shift loop): needs a u32 state field since interleaving two full u16s produces 32 bits, more than either input's own width. |
| `morton_decode` | `MortonDecode::run() -> u16` | Morton (Z-order curve) decode: the inverse of morton_encode — split a u32 spatial index back into its two interleaved u16 coordinates via the same branch-free bit-compaction trick (constant shift amounts, no dynamic-shift loop). |
| `bresenham_step` | `BresenhamStep::run() -> u16` | Bresenham line-drawing, one step: given the fixed line parameters (dx, dy — the absolute deltas between the endpoints) and the running error term (as a sign-magnitude pair, since state fields can't be i16 — err can go negative), reports whether this step advances x, y, or both (step_x/step_y, 0 or 1) and updates the error term. The caller applies step_x/step_y to its own x/y using its own known step directions (sx, sy) — tracking dx/dy/err here and x/y/sx/sy on the caller's side avoids needing four more sign-magnitude field pairs for quantities the error-term math never actually needs to know the sign of. Verified against a full reference line generator across 2,000 random line segments (coordinates up to +/-500) before shipping. |

## packing/BCD (4)

| id | signature | summary |
|---|---|---|
| `pack_u8` | `run(hi: u16, lo: u16) -> u16` | Pack two byte values into one u16: (hi << 8) \| lo. Each input masked to its low byte, so out-of-range inputs stay defined. |
| `pack_nibbles` | `run(hi: u16, lo: u16) -> u16` | Pack two 4-bit nibbles into one byte: (hi << 4) \| lo. Each input masked to its low nibble. |
| `bcd_encode` | `run(n: u16) -> u16` | Encode a two-digit decimal value (0-99) as packed BCD: tens in the high nibble, units in the low nibble. |
| `bcd_decode` | `run(bcd: u16) -> u16` | Decode a packed BCD byte (tens in the high nibble, units in the low nibble) back to its binary value. |

## vector (2)

| id | signature | summary |
|---|---|---|
| `dot2` | `Dot2::run() -> u16` | Dot product of two 2D vectors (ax, ay) and (bx, by): ax*bx + ay*by. |
| `norm2_sq` | `run(x: u16, y: u16) -> u16` | Squared magnitude of a 2D vector (x, y): x*x + y*y (no sqrt). |

## checked-arithmetic (28)

| id | signature | summary |
|---|---|---|
| `mul_u16_u16_to_u32` | `MulWide::run() -> u16` | Exact product of two u16 values as a wide u32 (never overflows: 65535*65535 fits u32). The math-campaign foundation cell — most checked arithmetic composes from this. |
| `add_checked_u32` | `AddChecked::run() -> u16` | Checked u32 add: escalates (needs_wider_math) instead of silently wrapping if a + b overflows u32. |
| `sub_checked_u32` | `SubChecked::run() -> u16` | Checked u32 subtract: escalates (needs_wider_math) instead of wrapping if b > a (the result would be negative). |
| `div_exact_u32` | `DivExact::run() -> u16` | Exact u32 division: escalates (needs_wider_math) if b is zero or a doesn't divide evenly by b — a wrong-plan signal for word problems that declared an exact division. |
| `div_floor_u32` | `DivFloor::run() -> u16` | Floor division of two u32 values: a / b, rounded down. Escalates (needs_wider_math) if b is zero. |
| `div_ceil_u32` | `DivCeil::run() -> u16` | Ceiling division of two u32 values: the smallest integer >= a / b. Escalates (needs_wider_math) if b is zero. |
| `mod_u32` | `ModU32::run() -> u16` | Remainder of two u32 values: a % b. Escalates (needs_wider_math) if b is zero. |
| `fits_u16` | `FitsU16::run() -> u16` | Returns 1 if a wide u32 value fits in u16 (<= 65535) without narrowing loss, else 0. |
| `mul_checked_u32` | `MulChecked::run() -> u16` | Checked u32 multiply: escalates (needs_wider_math) instead of wrapping if a * b overflows u32. |
| `mul_add_checked_u32` | `MulAddChecked::run() -> u16` | Checked fused multiply-add at u32: a*b+c, escalating on either the multiply or the add overflowing (e.g. a per-unit price times a quantity, plus a flat fee). |
| `mul_sub_checked_u32` | `MulSubChecked::run() -> u16` | Checked fused multiply-subtract at u32: a*b-c, escalating if the multiply overflows or c exceeds the product (e.g. a per-unit price times a quantity, minus a flat discount). |
| `mul3_checked_u32` | `Mul3Checked::run() -> u16` | Checked three-way multiply at u32: a*b*c, escalating if either multiply step overflows (e.g. a box volume: length*width*height). |
| `add3_checked_u32` | `Add3Checked::run() -> u16` | Checked three-way add at u32: a+b+c, escalating if either add step overflows — the exact, wide sibling of sum3 (which saturates at u16). |
| `pow_checked_u32` | `PowChecked::run() -> u16` | Checked exact exponentiation at u32: base^exp, escalating the moment a multiply step would overflow (distinct from pow_small, which saturates at u16 — this stays exact or hands off). 0^0 = 1. |
| `abs_diff_u32` | `AbsDiffWide::run() -> u16` | Absolute difference \|a - b\| between two wide u32 values — the exact wide sibling of abs_diff (which works over u16 and can't represent differences beyond 65535). |
| `min_u32` | `MinWide::run() -> u16` | Minimum of two wide u32 values — the exact wide sibling of min (which works over u16). |
| `max_u32` | `MaxWide::run() -> u16` | Maximum of two wide u32 values — the exact wide sibling of max (which works over u16). |
| `clamp_u32` | `ClampWide::run() -> u16` | Clamp a wide u32 value to the inclusive range [lo, hi] — the wide sibling of clamp (which works over u16). |
| `range_check_u32` | `RangeCheckWide::run() -> u16` | Returns 1 if lo <= x <= hi at wide u32 width, else 0 — the wide sibling of range_check (which works over u16). |
| `avg2_u32` | `Avg2Wide::run() -> u16` | Average of two wide u32 values, (a + b) / 2, computed without overflow — the wide sibling of avg2 (which works over u16). |
| `divides_u32` | `DividesWide::run() -> u16` | Returns 1 if a divides b evenly at wide u32 width (b % a == 0, a != 0), else 0 — the wide sibling of divides (which works over u16). |
| `gcd_u32` | `GcdWide::run() -> u16` | Greatest common divisor of two wide u32 values via an inline Euclidean loop — the wide sibling of gcd (which works over u16 and can't represent divisors beyond 65535). |
| `lcm_u32` | `LcmChecked::run() -> u16` | Least common multiple of two wide u32 values via an inline GCD (0 if either is 0), escalating on overflow — unlike lcm (u16, silently wraps on overflow), this is the exact, checked wide sibling. |
| `smag_add` | `SmagAdd::run() -> u16` | Sign-magnitude add: combine two signed quantities represented as (magnitude, sign) pairs — neg_a/neg_b are 0 (nonnegative) or 1 (negative), since the dialect has no i32 and this is how the math-campaign renderer tracks signed differences at u32 width (docs/math-campaign-spec.md). Escalates on magnitude overflow. |
| `smag_sub` | `SmagSub::run() -> u16` | Sign-magnitude subtract: a - b for two signed quantities represented as (magnitude, sign) pairs (neg 0=nonnegative, 1=negative, per smag_add) — computed by flipping b's sign and adding, the same rule table as smag_add. Escalates on magnitude overflow. |
| `smag_cmp` | `SmagCmp::run() -> u16` | Compare two signed quantities represented as (magnitude, sign) pairs (neg 0=nonnegative, 1=negative, per smag_add): 0 if a < b, 1 if equal, 2 if a > b — the sign-magnitude counterpart of frac_cmp's ordering-code convention. |
| `smag_mul` | `SmagMul::run() -> u16` | Multiply two signed values: magnitudes multiply (checked for overflow), sign is same-positive/different-negative (per smag_add). |
| `smag_div` | `SmagDiv::run() -> u16` | Divide two signed values exactly: magnitudes divide (escalating on a nonzero remainder), sign is same-positive/different-negative (per smag_add). |

## money-bps (8)

| id | signature | summary |
|---|---|---|
| `bps_of` | `BpsOf::run() -> u16` | Basis points of a wide value: value * bps / 10000 (e.g. 500 bps of 1000 is 50 — 5%). Escalates (needs_wider_math) on multiply overflow. |
| `increase_by_bps` | `IncreaseByBps::run() -> u16` | Increase a wide value by bps basis points (covers tax/tip/markup — same formula: value + value*bps/10000). Escalates on multiply or add overflow. |
| `decrease_by_bps` | `DecreaseByBps::run() -> u16` | Decrease a wide value by bps basis points (covers discount: value - value*bps/10000). Escalates if the discount would exceed the value, or on multiply overflow. |
| `original_before_bps_increase` | `OriginalBeforeIncrease::run() -> u16` | Recover the original value before a bps increase, given the final value: final * 10000 / (10000 + bps). The inverse of increase_by_bps. |
| `original_before_bps_decrease` | `OriginalBeforeDecrease::run() -> u16` | Recover the original value before a bps decrease, given the final value: final * 10000 / (10000 - bps). The inverse of decrease_by_bps. |
| `cents_mul_qty` | `CentsMulQty::run() -> u16` | Total price in cents (the minor unit of any decimal currency — cents, pence, kopecks, not USD specifically): unit_cents * qty. Escalates (needs_wider_math) on multiply overflow — distinct from mul_u16_u16_to_u32 (that one always fits u32 exactly; this one's unit_cents is already wide and can genuinely overflow). |
| `bps_increase_between` | `BpsIncreaseBetween::run() -> u16` | Infer the basis-points increase between two wide values: given before and after (after >= before), the rate = (after - before) * 10000 / before — the inverse of increase_by_bps (that computes the final value from a rate; this recovers the rate from the two values). |
| `bps_decrease_between` | `BpsDecreaseBetween::run() -> u16` | Infer the basis-points decrease between two wide values: given before and after (after <= before), the rate = (before - after) * 10000 / before — the inverse of decrease_by_bps (that computes the final value from a rate; this recovers the rate from the two values). |

## units (4)

| id | signature | summary |
|---|---|---|
| `same_unit_check` | `run(a: u16, b: u16) -> u16` | Unit-compatibility check for adding/subtracting two typed quantities: returns their shared dimension code if the units match, else escalates on a units mismatch (dimension codes documented in docs/library-growth.md, now including 8=rate_money_per_time and 9=rate_count_per_time). |
| `unit_mul` | `run(a: u16, b: u16) -> u16` | Resulting unit-dimension code when multiplying two typed quantities (e.g. count*money=money, distance*distance=area) — codes: 0=count,1=money,2=time,3=distance,4=area,5=volume,6=rate_money_per_count,7=rate_distance_per_time,8=rate_money_per_time,9=rate_count_per_time (docs/library-growth.md). Escalates on any unmodeled pair. |
| `unit_div` | `run(a: u16, b: u16) -> u16` | Resulting unit-dimension code when dividing a numerator quantity by a denominator quantity (e.g. money/count=rate_money_per_count, money/time=rate_money_per_time, count/time=rate_count_per_time, same/same=count) — same codes as unit_mul (docs/library-growth.md). Escalates on any unmodeled pair. |
| `unit_cancel_check` | `run(a: u16, b: u16) -> u16` | Returns 1 if dividing a numerator-unit quantity by a denominator-unit quantity is dimensionally defined (same rule table as unit_div), else 0 — a non-escalating probe for a caller (e.g. a plan verifier) trying several candidate unit pairs without halting. |

## verifier-ranker (16)

| id | signature | summary |
|---|---|---|
| `sum_equals` | `run(a: u16, b: u16, total: u16) -> u16` | Verifies a claimed sum: returns 1 if a + b == total, else 0 — computed in a wider internal width so a genuine overflow can't false-positive as a match on the wrapped value. |
| `diff_equals` | `run(a: u16, b: u16, remainder: u16) -> u16` | Verifies a claimed difference: returns 1 if a >= b and a - b == remainder, else 0 (including when a < b, since an unsigned difference can't be negative). |
| `product_equals_u32` | `ProductEquals::run() -> u16` | Verifies a claimed wide product: 1 if a * b == total, else 0 — including when a * b overflows u32 (a real overflow just means the claim doesn't hold, not an escalation; a verifier always returns a verdict). |
| `quotient_equals_exact_u32` | `QuotientEqualsExact::run() -> u16` | Verifies a claimed exact wide quotient: 1 if b != 0, a divides evenly by b (a % b == 0), and a / b == quotient, else 0 — the verifier counterpart of div_exact_u32 (that one computes and escalates on a remainder; this one checks a candidate answer and always returns a verdict). |
| `answer_eq_u32` | `AnswerEqWide::run() -> u16` | Verifies a claimed wide answer: returns 1 if a == b, else 0 — the wide sibling of eq (which works over u16 and can't compare values beyond 65535, e.g. money totals in cents). |
| `sum_equals_u32` | `SumEqualsWide::run() -> u16` | Verifies a claimed wide sum: returns 1 if a + b == total, else 0, without escalating on overflow (a real overflow just means the claim doesn't hold) — the wide sibling of sum_equals (which works over u16). |
| `diff_equals_u32` | `DiffEqualsWide::run() -> u16` | Verifies a claimed wide difference: returns 1 if a >= b and a - b == remainder, else 0 (including when a < b, since an unsigned difference can't be negative) — the wide sibling of diff_equals (which works over u16). |
| `sum3_equals_u32` | `Sum3EqualsWide::run() -> u16` | Verifies a claimed wide three-way sum: returns 1 if a + b + c == total, else 0, without escalating on overflow — the reverse-equation counterpart of add3_checked_u32. |
| `product3_equals_u32` | `Product3EqualsWide::run() -> u16` | Verifies a claimed wide three-way product: returns 1 if a * b * c == total, else 0, including when the product overflows u32 (a real overflow just means the claim doesn't hold) — the reverse-equation counterpart of mul3_checked_u32. |
| `mul_add_equals_u32` | `MulAddEqualsWide::run() -> u16` | Verifies a claimed wide fused multiply-add: returns 1 if a * b + c == total, else 0, including when either step overflows u32 — the reverse-equation counterpart of mul_add_checked_u32. |
| `mul_sub_equals_u32` | `MulSubEqualsWide::run() -> u16` | Verifies a claimed wide fused multiply-subtract: returns 1 if a * b - c == total, else 0, including when the multiply overflows u32 or c exceeds the product — the reverse-equation counterpart of mul_sub_checked_u32. |
| `pow_equals_u32` | `PowEqualsWide::run() -> u16` | Verifies a claimed wide power: returns 1 if base^exp == total, else 0, including when an intermediate multiply overflows u32 — the reverse-equation counterpart of pow_checked_u32. |
| `smag_is_nonneg` | `SmagIsNonneg::run() -> u16` | Constraint check for a signed-magnitude quantity (magnitude, sign pair — neg 0=nonnegative, 1=negative, per smag_add): returns 1 if the value is nonnegative (neg == 0, or magnitude == 0 regardless of the sign flag), else 0. |
| `agree3_u32` | `Agree3Wide::run() -> u16` | Multi-plan agreement check at wide u32 width: returns 1 if at least two of three candidate answers are equal, else 0 — the wide sibling of majority3 (which works over u16 and can't represent answers beyond 65535, e.g. money totals in cents). |
| `answer_within_tolerance_u32` | `AnswerWithinToleranceWide::run() -> u16` | Verifies a claimed wide answer is within an absolute tolerance of the true value: returns 1 if \|candidate - actual\| <= tolerance, else 0 — distinct from within_percent (a percentage-based tolerance over u16); this is an absolute margin at wide u32 width. |
| `smag_eq` | `SmagEq::run() -> u16` | Verifies whether two signed values (magnitude, sign pairs, per smag_add) are equal, canonicalizing negative-zero to nonnegative first — the sign-magnitude counterpart of frac_eq / answer_eq_u32. |

## stateful/RNG (3)

| id | signature | summary |
|---|---|---|
| `lcg_next` | `Lcg::run() -> u16` | Linear congruential generator step: seed = seed * 1664525 + 1013904223 (mod 2^32, Numerical Recipes constants), returning the top 16 bits (the higher bits of an LCG are far less patterned than the low bits). The caller threads `seed` through — re-supply the field each call, since state cells don't persist memory across separate runs. |
| `xorshift16` | `Xorshift16::run() -> u16` | 16-bit xorshift generator step (x ^= x<<7; x ^= x>>9; x ^= x<<8) — a distinct pseudo-random recurrence from lcg_next (no multiply, pure shift/xor). The caller threads `x` through — re-supply the field each call. A seed of 0 is a fixed point (0 forever); always seed nonzero. |
| `counter_step` | `CounterStep::run() -> u16` | Modular counter step: increments count by 1, wrapping to 0 the moment it would reach `limit` (limit 0 means never wrap — a plain saturating-free incrementer). Useful for round-robin dispatch or a bounded retry index. The caller threads `count` through — re-supply the field each call. |

## signed-deltas (4)

| id | signature | summary |
|---|---|---|
| `sign_i16` | `run(x: i16) -> i16` | Sign of a signed value: 1 if positive, -1 if negative, 0 if zero. |
| `abs_i16` | `run(x: i16) -> u16` | Absolute value of a signed 16-bit value, returned as u16 (correctly handles i16::MIN, whose magnitude 32768 doesn't fit back in i16). |
| `clamp_i16` | `run(x: i16, lo: i16, hi: i16) -> i16` | Clamp a signed value to the inclusive range [lo, hi] — the signed counterpart of clamp (which only works over u16). Also the exact form of "hard tanh" in Q8.8 fixed point (clamp_i16(x, -256, 256)): tanh_hard(x) = 2*sigmoid_hard(2x)-1 reduces algebraically to clamp(x, -1, 1), so q_tanh was deliberately not shipped as a second cell — same formula, different name, exactly the case the admission gate exists to catch. |
| `apply_delta_clamped` | `run(value: u16, delta: i16, cap: u16) -> u16` | Apply a signed delta to an unsigned value, clamped to [0, cap] — e.g. a health/resource/score adjustment that can't go negative or exceed a cap (a "risk delta" applied safely). |

## fractions (21)

| id | signature | summary |
|---|---|---|
| `frac_reduce` | `FracReduce::run() -> u16` | Reduce a fraction n/d to lowest terms via an inline Euclidean GCD (no shared gcd_u32 helper — a two-u32-param function still can't cross a call boundary, so the loop is duplicated in every fraction cell that needs it). |
| `frac_add` | `FracAdd::run() -> u16` | Add two fractions na/da + nb/db, reduced to lowest terms via an inline GCD. |
| `frac_sub` | `FracSub::run() -> u16` | Subtract two fractions na/da - nb/db, reduced to lowest terms via an inline GCD. |
| `frac_mul` | `FracMul::run() -> u16` | Multiply two fractions na/da * nb/db, reduced to lowest terms via an inline GCD. |
| `frac_div` | `FracDiv::run() -> u16` | Divide two fractions (na/da) / (nb/db) = (na*db)/(da*nb), reduced to lowest terms via an inline GCD. |
| `frac_cmp` | `FracCmp::run() -> u16` | Compare two fractions na/da vs nb/db via cross-multiplication (works on unreduced fractions, e.g. 1/2 vs 2/4): 0 if less, 1 if equal, 2 if greater. |
| `frac_eq` | `FracEq::run() -> u16` | Returns 1 if two fractions na/da and nb/db are equal, else 0 — via cross-multiplication, so unreduced-but-equivalent fractions (e.g. 1/2 vs 2/4) still compare equal without needing to reduce first. |
| `is_integer` | `IsInteger::run() -> u16` | Returns 1 if the wide fraction n/d is a whole number (n divides evenly by d), else 0 — a wrong-plan signal for word problems that expect an exact split. |
| `frac_to_mixed` | `FracToMixed::run() -> u16` | Convert an improper fraction n/d to a mixed number: whole + num/den, where the remaining fraction is reduced to lowest terms via an inline GCD (num=0, den=1 if n divides evenly by d). |
| `ratio_split2` | `RatioSplit2::run() -> u16` | Split a wide total into two parts in a given ratio (ratio_a : ratio_b): part_a = total*ratio_a/(ratio_a+ratio_b), part_b = total - part_a — guaranteed to sum exactly to total (the remainder from integer division always lands on part_b), unlike computing both parts independently. |
| `frac_reciprocal` | `FracReciprocal::run() -> u16` | Reciprocal of a fraction n/d: swaps to d/n. Escalates (halt 0xFF06, out_of_domain) if n == 0 (a zero fraction has no reciprocal) or d == 0 (not a valid fraction to begin with). |
| `frac_of_whole` | `FracOfWhole::run() -> u16` | A fraction of a whole number, computed exactly: n/d * whole, escalating if it doesn't divide evenly (a wrong-plan signal — e.g. "3/4 of 20" should be exact for a grade-school word problem) or if the multiply overflows. |
| `frac_scale` | `FracScale::run() -> u16` | Scale a fraction by an integer: (n/d) * k, reduced to lowest terms via an inline GCD — unlike frac_of_whole (which requires an exact whole-number result), this always stays a fraction. |
| `frac_min` | `FracMin::run() -> u16` | The smaller of two fractions na/da and nb/db, by cross-multiplication (works on unreduced fractions) — returns its numerator/denominator as given (ties keep na/da). Distinct from frac_cmp, which only returns an ordering code, not the winning fraction itself. |
| `frac_max` | `FracMax::run() -> u16` | The larger of two fractions na/da and nb/db, by cross-multiplication (works on unreduced fractions) — returns its numerator/denominator as given (ties keep na/da). Distinct from frac_cmp, which only returns an ordering code, not the winning fraction itself. |
| `ratio_split3` | `RatioSplit3::run() -> u16` | Split a wide total three ways by a given ratio (ratio_a : ratio_b : ratio_c): part_a and part_b get their proportional share by integer division, part_c takes the remainder — guaranteed to sum exactly to total (the direct 3-way sibling of ratio_split2). |
| `frac_is_proper` | `FracIsProper::run() -> u16` | Returns 1 if a fraction n/d is proper (n < d, i.e. less than one whole), else 0. Escalates (halt 0xFF06, out_of_domain) if d == 0. |
| `frac_add_whole` | `FracAddWhole::run() -> u16` | Add a whole number to a fraction: n/d + whole = (n + whole*d)/d, reduced to lowest terms via an inline GCD. |
| `mixed_to_frac` | `MixedToFrac::run() -> u16` | Convert a mixed number (whole + num/den) to a single improper fraction: n = whole*den + num, d = den — the exact inverse of frac_to_mixed. |
| `frac_avg2` | `FracAvg2::run() -> u16` | Average of two fractions na/da and nb/db, reduced to lowest terms via an inline GCD. |
| `frac_sub_from_whole` | `FracSubFromWhole::run() -> u16` | Subtract a fraction from a whole number: whole - n/d, reduced to lowest terms via an inline GCD. |

## combinatorics (6)

| id | signature | summary |
|---|---|---|
| `factorial_checked_u32` | `FactorialChecked::run() -> u16` | Factorial of n, checked: n! — escalates instead of silently wrapping once n! would exceed u32::MAX (n >= 13, since 13! overflows u32). |
| `choose_u32` | `ChooseWide::run() -> u16` | Binomial coefficient "n choose k" (nCr), checked: the count of k-element subsets of an n-element set, via the multiplicative running-division formula (each step's quotient is always exact, but the pre-division product can transiently exceed the final answer, so this escalates somewhat before n choose k itself would overflow u32 — a known limitation of single-pass 32-bit intermediates, not a false claim). Escalates rather than silently wrapping. |
| `permute_u32` | `PermuteWide::run() -> u16` | Permutations "n pick k" (nPr): the count of ordered k-element selections from an n-element set, n!/(n-k)! computed directly as a product of k descending terms (never materializing the full factorials). Escalates on overflow rather than silently wrapping. |
| `fibonacci_checked_u32` | `FibonacciChecked::run() -> u16` | The nth Fibonacci number (F(0)=0, F(1)=1, F(n)=F(n-1)+F(n-2)), checked: escalates instead of silently wrapping once F(n) would exceed u32::MAX (n >= 47). |
| `catalan_number` | `CatalanNumber::run() -> u16` | The nth Catalan number (C(0)=1, C(n+1) = C(n)*2*(2n+1)/(n+2) — an exact recurrence, each step's division always lands evenly), checked: escalates on overflow rather than silently wrapping. Note the recurrence's own pre-division intermediate can overflow u32 before the true Catalan number itself would (the same class of limitation choose_u32 documents) — verified safe through C(17); beyond that, escalation is possible even though C(18)/C(19) themselves would still fit u32. |
| `derangement_count` | `DerangementCount::run() -> u16` | The nth derangement number (D(0)=1, D(1)=0, D(n)=(n-1)*(D(n-1)+D(n-2)) — the count of permutations of n items with no fixed point), checked: escalates instead of silently wrapping once D(n) would exceed u32::MAX (n >= 14). Unlike catalan_number's recurrence, this one's intermediate never overflows before the true result itself would (verified) — the multiplier grows linearly (n-1) against a linearly-combined sum, not against an already-exponential value. |

## geometry (3)

| id | signature | summary |
|---|---|---|
| `shoelace_area_x2` | `ShoelaceAreaX2::run() -> u16` | Twice the area of a triangle from three integer vertices (x1,y1),(x2,y2),(x3,y3), via the shoelace formula: \|x1*(y2-y3) + x2*(y3-y1) + x3*(y1-y2)\| — always an integer, unlike the raw area (which is a half-integer for e.g. a right triangle with legs 1 and 1). Coordinates are unsigned; the three (y-difference)*(x-coordinate) terms are combined as sign-magnitude values inline (no shared smag_* subroutine call — a u32 value still can't cross more than one call boundary), since a term or the running sum can go negative before the final absolute value. |
| `shoelace_area_x2_quad` | `ShoelaceAreaX2Quad::run() -> u16` | Twice the area of a quadrilateral from four integer vertices (x1,y1)..(x4,y4), generalizing shoelace_area_x2's triangle formula to \|x1*(y2-y4) + x2*(y3-y1) + x3*(y4-y2) + x4*(y1-y3)\| — always an integer. Coordinates are unsigned; the four (y-difference)*(x-coordinate) terms are combined as sign-magnitude values inline (no shared smag_* subroutine call — a u32 value still can't cross more than one call boundary), the same pattern shoelace_area_x2 uses, extended to a fourth term. |
| `triangle_is_valid` | `run(a: u16, b: u16, c: u16) -> u16` | Returns 1 if three side lengths (a, b, c) form a valid (non-degenerate) triangle, i.e. each side is strictly less than the sum of the other two, else 0. Sums are widened to u32 internally so a large pair (e.g. two sides near 65535) can't wrap past u16 and silently flip the verdict. |

## sequences (2)

| id | signature | summary |
|---|---|---|
| `arithmetic_series_sum` | `ArithmeticSeriesSum::run() -> u16` | Sum of the first n terms of an arithmetic sequence starting at a with common difference d: n*(2a + (n-1)*d) / 2 — always an exact integer (the product n*(2a+(n-1)*d) is provably always even), checked for overflow at each step. |
| `geometric_series_sum` | `GeometricSeriesSum::run() -> u16` | Sum of the first n terms of a geometric sequence starting at a with ratio r (a + a*r + a*r^2 + ... + a*r^(n-1)), computed by direct iterative summation rather than the a*(r^n-1)/(r-1) closed form — r^n alone would overflow long before a genuinely unrepresentable sum does, so this escalates exactly when the true sum (or an intermediate term) doesn't fit u32, no earlier. Exact for any r >= 0, not just r > 1. |

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
