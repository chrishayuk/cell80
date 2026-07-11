//! Verifies a claimed wide integer square root: recomputes the largest r with r*r <= n via the same branch-free bitwise loop isqrt_u32 runs internally, returning 1 if it equals the claimed r, else 0 -- the reverse-equation counterpart of isqrt_u32 (never halts, always a verdict).
//! tags: verify, verifier, equation, isqrt, sqrt, square-root, root, wide, u32, check, plan, reverse-equation, number-theory
//! entry: IsqrtEqualsWide::run
struct IsqrtEqualsWide { n: u32, r: u32 }
impl IsqrtEqualsWide {
    fn run(&mut self) -> u16 {
        let mut val = self.n;
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
        (res == self.r) as u16
    }
}
