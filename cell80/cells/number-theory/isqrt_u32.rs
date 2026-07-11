//! Integer square root at wide u32 width: the largest r with r*r <= n, for n up to u32::MAX — the wide sibling of isqrt (which is bounded to the u16 domain, n <= 65535). Uses the same branch-free bitwise integer-sqrt loop q_sqrt.rs runs internally on a u32 local, rather than isqrt's linear scan, since a linear scan over the full u32 domain would run tens of thousands of iterations past the default cycle budget.
//! tags: number, sqrt, square-root, isqrt, root, math, wide, u32, large, number-theory
//! entry: IsqrtWide::run
struct IsqrtWide { n: u32, r: u32 }
impl IsqrtWide {
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
        self.r = res;
        res as u16
    }
}
