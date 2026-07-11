//! Integer quadratic mean (root-mean-square) of two u16 values, floor(sqrt((a*a+b*b)/2)) -- the fourth classical Pythagorean mean alongside avg2 (arithmetic) and geomean2 (geometric), missing from safe-arith until now, computed by widening a*a and b*b to u32, combining via the shared add_checked_u32 kernel (escalates only when a*a+b*b itself exceeds u32::MAX), floor-dividing the checked sum by 2, then reducing with the same branch-free bitwise integer-sqrt loop geomean2/euclid_dist already run inline.
//! tags: math, mean, average, quadratic-mean, rms, root-mean-square, sqrt, isqrt, root, checked, safe-arith
//! limits: escalates (halt 0xFF05, needs_wider_math) if a*a + b*b exceeds u32::MAX (both inputs large simultaneously)
fn run(a: u16, b: u16) -> u16 {
    let asq = a as u32 * a as u32;
    let bsq = b as u32 * b as u32;
    let sum = add_checked_u32(asq, bsq);
    let half = sum / 2u32;

    // Branch-free bitwise integer square root of half (the same loop q_sqrt/isqrt_u32/geomean2/euclid_dist run inline).
    let mut val = half;
    let mut res = 0u32;
    let mut bit = 1u32 << 30u32;
    while bit > val { bit = bit >> 2u32; }
    while bit != 0u32 {
        if val >= res + bit {
            val = val - (res + bit);
            res = (res >> 1u32) + bit;
        } else {
            res = res >> 1u32;
        }
        bit = bit >> 2u32;
    }

    res as u16
}
