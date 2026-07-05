//! Permutations "n pick k" (nPr): the count of ordered k-element selections from an n-element set, n!/(n-k)! computed directly as a product of k descending terms (never materializing the full factorials). Escalates on overflow rather than silently wrapping.
//! tags: math, combinatorics, permute, permutation, nPr, counting, checked, wide, u32, escalate
//! entry: PermuteWide::run
//! limits: returns 0 if k > n; escalates (halt 0xFF05, needs_wider_math) if an intermediate multiply overflows u32
struct PermuteWide { n: u32, k: u32, result: u32 }
impl PermuteWide {
    fn run(&mut self) -> u16 {
        if self.k > self.n {
            self.result = 0u32;
            return 1u16;
        }
        let mut r = 1u32;
        let mut i = 0u32;
        while i < self.k {
            let term = self.n - i;
            let p = r.wrapping_mul(term);
            if r != 0u32 && p / r != term { halt(0xFF05u16); }
            r = p;
            i = i + 1u32;
        }
        self.result = r;
        1u16
    }
}
