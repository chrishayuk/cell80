//! Integer geometric mean of two u16 values, floor(sqrt(a*b)) -- the mean-family sibling avg2 (arithmetic mean) has none of, computed via the same branch-free bitwise integer-sqrt loop isqrt_u32/cosine_score_approx use, inlined here (cells can't call each other) on the widened u32 product a*b, always safe since two u16-bounded factors' product always fits u32 and the result always fits u16.
//! tags: math, mean, average, geometric-mean, sqrt, isqrt, root, product, safe-arith
fn run(a: u16, b: u16) -> u16 {
    let prod = a as u32 * b as u32;

    // Branch-free bitwise integer square root of prod (the same loop q_sqrt/isqrt_u32 run).
    let mut val = prod;
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
