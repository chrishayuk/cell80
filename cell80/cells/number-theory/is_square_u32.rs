//! Returns 1 if n (u32) is a perfect square, else 0 -- the wide sibling of is_square (which works over u16, up to 65535). Finds the largest r with r*r <= n via an inlined binary search over r in [0, 65535] (the largest r whose square still fits in u32), then compares r*r to n -- the same search isqrt_u32 does internally, inlined here since cells can't call each other; binary search keeps this cheap across the whole u32 domain, unlike a linear scan.
//! tags: number, square, perfect-square, predicate, sqrt, root, wide, u32, isqrt
//! entry: IsSquareWide::run
struct IsSquareWide { n: u32, result: u16 }
impl IsSquareWide {
    fn run(&mut self) -> u16 {
        let mut lo = 0u32;
        let mut hi = 65535u32;
        while lo < hi {
            let mid = (lo + hi + 1u32) / 2u32;
            if mid * mid <= self.n { lo = mid; } else { hi = mid - 1u32; }
        }
        let v = (lo * lo == self.n) as u16;
        self.result = v;
        v
    }
}
