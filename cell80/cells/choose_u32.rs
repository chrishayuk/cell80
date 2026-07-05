//! Binomial coefficient "n choose k" (nCr), checked: the count of k-element subsets of an n-element set, via the multiplicative running-division formula (each step's quotient is always exact, but the pre-division product can transiently exceed the final answer, so this escalates somewhat before n choose k itself would overflow u32 — a known limitation of single-pass 32-bit intermediates, not a false claim). Escalates rather than silently wrapping.
//! tags: math, combinatorics, choose, binomial, combination, nCr, counting, checked, wide, u32, escalate
//! entry: ChooseWide::run
//! limits: returns 0 if k > n; escalates (halt 0xFF05, needs_wider_math) if an intermediate product overflows u32 — this can trigger before the true n-choose-k result would itself exceed u32::MAX
struct ChooseWide { n: u32, k: u32, result: u32 }
impl ChooseWide {
    fn run(&mut self) -> u16 {
        if self.k > self.n {
            self.result = 0u32;
            return 1u16;
        }
        let mut kk = self.k;
        if self.n - self.k < self.k { kk = self.n - self.k; }
        let mut r = 1u32;
        let mut i = 1u32;
        while i <= kk {
            let term = self.n - kk + i;
            let num = r.wrapping_mul(term);
            if r != 0u32 && num / r != term { halt(0xFF05u16); }
            r = num / i;
            i = i + 1u32;
        }
        self.result = r;
        1u16
    }
}
