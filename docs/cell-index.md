# Cell index — every landed cell, by pack

*Generated from `cell80/cells` (145 cells) by `cell80/scripts/gen_cell_index.py`. Regenerate after any cell is added/removed:*

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

## number-theory (17)

| id | signature | summary |
|---|---|---|
| `lcm` | `run(a: u16, b: u16) -> u16` | Least common multiple of two values (a/gcd*b; 0 if either is 0). u16 domain. |
| `gcd` | `run(a: u16, b: u16) -> u16` | Greatest common divisor (Euclid's algorithm). |
| `gcd3` | `run(a: u16, b: u16, c: u16) -> u16` | Greatest common divisor of three values. |
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

## scoring/choice (2)

| id | signature | summary |
|---|---|---|
| `weighted_sum` | `run(a: u16, b: u16, c: u16) -> u16` | Weighted sum of three inputs with fixed weights 1, 2, 3 (a candidate score). |
| `weighted_sum_wide` | `Ws::run() -> u16` | Exact weighted sum with a wide u32 result field: sum = a + 2b + 3c, no u16 wrap (sibling of weighted_sum). |

## calendrical/checksum (4)

| id | signature | summary |
|---|---|---|
| `is_leap_year` | `run(year: u16) -> u16` | Returns 1 if year is a Gregorian leap year, else 0: divisible by 4, except centuries not divisible by 400. |
| `days_in_month` | `run(month: u16, is_leap: u16) -> u16` | Number of days in a month (1-12; 0 for an invalid month), given a leap-year flag for February. |
| `day_of_week` | `run(year: u16, month: u16, day: u16) -> u16` | Day of week for a Gregorian date via Zeller's congruence: 0=Saturday, 1=Sunday, 2=Monday, ... 6=Friday. |
| `luhn_check` | `run(n: u16) -> u16` | Returns 1 if n's decimal digits pass the Luhn checksum (mod 10, doubling every second digit from the right), else 0. |

## fixed-point (3)

| id | signature | summary |
|---|---|---|
| `q_mul` | `run(a: u16, b: u16) -> u16` | Q8.8 fixed-point multiply: (a * b) >> 8, computed wide so the 16.16 intermediate doesn't overflow. |
| `q_div` | `run(a: u16, b: u16) -> u16` | Q8.8 fixed-point divide: (a << 8) / b, returning 0 when b == 0 (no divide-by-zero). |
| `q_lerp` | `run(a: u16, b: u16, t: u16) -> u16` | Linear interpolation from a to b by t (Q0.8 fraction, 0..256 = 0.0..1.0): a + (b-a)*t/256. Also an EMA step: q_lerp(prev, sample, alpha). |

## agentic-runtime (5)

| id | signature | summary |
|---|---|---|
| `token_bucket_step` | `TokenBucket::run() -> u16` | Token-bucket rate limiter step: refill by `refill`, cap at `capacity`, then try to spend `cost`; 1 if allowed, 0 if not enough tokens (tokens still refill either way). |
| `backoff_next` | `Backoff::run() -> u16` | Capped exponential backoff: next = min(current * 2, cap), starting at 1 when current is 0. |
| `circuit_breaker_step` | `CircuitBreaker::run() -> u16` | Circuit-breaker state machine step: closed(0) counts failures and opens at the threshold; open(1) waits for cooldown then tries half-open(2); half-open resolves to closed on success or back to open on failure. |
| `debounce_step` | `Debounce::run() -> u16` | Debounce a noisy 0/1 signal: only confirms a change to `input` once it's held for `threshold` consecutive steps; output is the last confirmed-stable value. |
| `hysteresis` | `Hysteresis::run() -> u16` | Hysteresis (Schmitt-trigger) state: turns on at value >= high, turns off at value <= low, else holds the prior state (the dead zone between them). |

## running-stats (3)

| id | signature | summary |
|---|---|---|
| `running_min_max_step` | `RunningMinMax::run() -> u16` | Running min/max tracker over a stream of values: updates min/max (self-initializing on the first call via `seen`), returns the current range (max - min). |
| `streak_step` | `Streak::run() -> u16` | Consecutive-streak counter: increments while input is nonzero, resets to 0 the moment input is 0. |
| `accumulate_step` | `Accumulate::run() -> u16` | Running sum + count over a stream of values (sum saturates at 65535). Compose with safe_div(sum, count) for a running mean. |

## spatial/grid (3)

| id | signature | summary |
|---|---|---|
| `grid_index` | `run(x: u16, y: u16, width: u16) -> u16` | Flat array index of a grid cell (x, y) in a grid of the given row width: y * width + x. |
| `point_in_rect` | `PointInRect::run() -> u16` | Returns 1 if point (px, py) is inside rect (rx, ry, rw, rh) — half-open: [rx, rx+rw) x [ry, ry+rh) — else 0. |
| `aabb_intersect` | `AabbIntersect::run() -> u16` | Returns 1 if two axis-aligned bounding boxes (x1,y1,w1,h1) and (x2,y2,w2,h2) overlap (edge-touching doesn't count), else 0. |

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

## checked-arithmetic (8)

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

## money-bps (6)

| id | signature | summary |
|---|---|---|
| `bps_of` | `BpsOf::run() -> u16` | Basis points of a wide value: value * bps / 10000 (e.g. 500 bps of 1000 is 50 — 5%). Escalates (needs_wider_math) on multiply overflow. |
| `increase_by_bps` | `IncreaseByBps::run() -> u16` | Increase a wide value by bps basis points (covers tax/tip/markup — same formula: value + value*bps/10000). Escalates on multiply or add overflow. |
| `decrease_by_bps` | `DecreaseByBps::run() -> u16` | Decrease a wide value by bps basis points (covers discount: value - value*bps/10000). Escalates if the discount would exceed the value, or on multiply overflow. |
| `original_before_bps_increase` | `OriginalBeforeIncrease::run() -> u16` | Recover the original value before a bps increase, given the final value: final * 10000 / (10000 + bps). The inverse of increase_by_bps. |
| `original_before_bps_decrease` | `OriginalBeforeDecrease::run() -> u16` | Recover the original value before a bps decrease, given the final value: final * 10000 / (10000 - bps). The inverse of decrease_by_bps. |
| `cents_mul_qty` | `CentsMulQty::run() -> u16` | Total price in cents (the minor unit of any decimal currency — cents, pence, kopecks, not USD specifically): unit_cents * qty. Escalates (needs_wider_math) on multiply overflow — distinct from mul_u16_u16_to_u32 (that one always fits u32 exactly; this one's unit_cents is already wide and can genuinely overflow). |

## units (4)

| id | signature | summary |
|---|---|---|
| `same_unit_check` | `run(a: u16, b: u16) -> u16` | Unit-compatibility check for adding/subtracting two typed quantities: returns their shared dimension code if a == b, else escalates — codes: 0=count,1=money,2=time,3=distance,4=area,5=volume,6=rate_money_per_count,7=rate_distance_per_time (docs/library-growth.md). |
| `unit_mul` | `run(a: u16, b: u16) -> u16` | Resulting unit-dimension code when multiplying two typed quantities (e.g. count*money=money, distance*distance=area) — codes: 0=count,1=money,2=time,3=distance,4=area,5=volume,6=rate_money_per_count,7=rate_distance_per_time (docs/library-growth.md). Escalates on any unmodeled pair. |
| `unit_div` | `run(a: u16, b: u16) -> u16` | Resulting unit-dimension code when dividing a numerator quantity by a denominator quantity (e.g. money/count=rate_money_per_count, same/same=count) — same codes as unit_mul (docs/library-growth.md). Escalates on any unmodeled pair. |
| `unit_cancel_check` | `run(a: u16, b: u16) -> u16` | Returns 1 if dividing a numerator-unit quantity by a denominator-unit quantity is dimensionally defined (same rule table as unit_div), else 0 — a non-escalating probe for a caller (e.g. a plan verifier) trying several candidate unit pairs without halting. |

## verifier-ranker (4)

| id | signature | summary |
|---|---|---|
| `sum_equals` | `run(a: u16, b: u16, total: u16) -> u16` | Verifies a claimed sum: returns 1 if a + b == total, else 0 — computed in a wider internal width so a genuine overflow can't false-positive as a match on the wrapped value. |
| `diff_equals` | `run(a: u16, b: u16, remainder: u16) -> u16` | Verifies a claimed difference: returns 1 if a >= b and a - b == remainder, else 0 (including when a < b, since an unsigned difference can't be negative). |
| `product_equals_u32` | `ProductEquals::run() -> u16` | Verifies a claimed wide product: 1 if a * b == total, else 0 — including when a * b overflows u32 (a real overflow just means the claim doesn't hold, not an escalation; a verifier always returns a verdict). |
| `quotient_equals_exact_u32` | `QuotientEqualsExact::run() -> u16` | Verifies a claimed exact wide quotient: 1 if b != 0, a divides evenly by b (a % b == 0), and a / b == quotient, else 0 — the verifier counterpart of div_exact_u32 (that one computes and escalates on a remainder; this one checks a candidate answer and always returns a verdict). |

## stateful/RNG (3)

| id | signature | summary |
|---|---|---|
| `lcg_next` | `Lcg::run() -> u16` | Linear congruential generator step: seed = seed * 1664525 + 1013904223 (mod 2^32, Numerical Recipes constants), returning the top 16 bits (the higher bits of an LCG are far less patterned than the low bits). The caller threads `seed` through — re-supply the field each call, since state cells don't persist memory across separate runs. |
| `xorshift16` | `Xorshift16::run() -> u16` | 16-bit xorshift generator step (x ^= x<<7; x ^= x>>9; x ^= x<<8) — a distinct pseudo-random recurrence from lcg_next (no multiply, pure shift/xor). The caller threads `x` through — re-supply the field each call. A seed of 0 is a fixed point (0 forever); always seed nonzero. |
| `counter_step` | `CounterStep::run() -> u16` | Modular counter step: increments count by 1, wrapping to 0 the moment it would reach `limit` (limit 0 means never wrap — a plain saturating-free incrementer). Useful for round-robin dispatch or a bounded retry index. The caller threads `count` through — re-supply the field each call. |

## aliases (4)

Behaviourally identical to a landed cell (found by the Phase 2.2 admission gate); removed as separate code, vocabulary merged into the surviving cell's tags.

| old id | → | landed as |
|---|---|---|
| `argmin2` | → | `is_gt` |
| `argmax2` | → | `is_lt` |
| `quantize` | → | `safe_div` |
| `wrap` | → | `safe_mod` |

## planned (not yet landed)

See `docs/library-growth.md` "Next waves" for the prioritized list (stateful/RNG above is the first slice landed — bounded_rand deferred, an exact duplicate of safe_mod; time/budget and signed deltas still ahead), the Phase 2.3 pilot-batch section for the author->verify->admit loop, and `docs/math-campaign-spec.md` for the GSM8K math campaign (M1: checked-arithmetic, money-bps, units, and verifier-ranker above are the first four authored packs; fractions is the last, gated on M0's u32-across-a-call-boundary compiler feature, confirmed still unbuilt this session even after Cond32 landed). All five originally-planned wave-3 packs plus the Phase 2.3 pilot batch (packing/BCD, vector) landed a first slice above. `unpack_lo`/`unpack_hi` were never built — checking docs/cell-index.md before authoring found they'd be exact duplicates of `low_byte`/`high_byte`. Each first slice deferred its harder items: ISBN/IBAN/UPC checksums need a wider-than-u32 input (see library-growth.md); q_sqrt/piecewise sigmoid-tanh, rate_window_update, a fixed-point running variance (Welford), Morton encode/decode (needs a u32 state field, not yet risked), a Bresenham stepper, and cosine_score_approx (deferred: exact fixed-point cosine needs a wide sqrt-of-a-product without overflow, not yet worked out) are all still open.
