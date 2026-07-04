# Cell index — every landed cell, by pack

*Generated from `cell80/cells` (96 cells) by `cell80/scripts/gen_cell_index.py`. Regenerate after any cell is added/removed:*

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

## aliases (4)

Behaviourally identical to a landed cell (found by the Phase 2.2 admission gate); removed as separate code, vocabulary merged into the surviving cell's tags.

| old id | → | landed as |
|---|---|---|
| `argmin2` | → | `is_gt` |
| `argmax2` | → | `is_lt` |
| `quantize` | → | `safe_div` |
| `wrap` | → | `safe_mod` |

## planned (not yet landed)

See `docs/library-growth.md` "Next waves" for the prioritized list (packing/BCD, vector, stateful/RNG, time/budget, signed deltas) and the roadmap discussion for the larger wave-3+ packs (fixed-point Q-format, agentic runtime primitives, calendrical/checksum validation, running statistics, spatial/grid).
