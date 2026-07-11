//! Returns 1 if n is a pronic (oblong) number, n = k*(k+1) for some integer k >= 0 (0, 2, 6, 12, 20, 30, ...), else 0 -- equivalent to 4n+1 being a perfect square, checked via the same branch-free bitwise integer-sqrt loop isqrt_u32/cosine_score_approx use, inlined here (cells can't call each other) on a u32 local formed from n.
//! tags: number, pronic, oblong, rectangular, heteromecic, predicate, sqrt, isqrt, number-theory
fn run(n: u16) -> u16 {
    let target = 4u32 * (n as u32) + 1u32;

    // Branch-free bitwise integer square root of target (the same loop q_sqrt/isqrt_u32 run).
    let mut val = target;
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

    (res * res == target) as u16
}
