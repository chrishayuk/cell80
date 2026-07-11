//! Lah number L(n,k) = C(n-1,k-1) * n!/k! for 1<=k<=n (L(0,0)=1 by convention, 0 otherwise): the count of ways to partition an n-element set into k nonempty linearly-ordered lists -- a third partition shape distinct from stirling_second (unordered subsets) and fubini_number (ordered sequence of unordered subsets), since here the blocks themselves are internally ordered but the collection of blocks is not. Computed as C(n-1,k-1) via the same multiplicative running-division formula choose_u32/stirling_second use, times n!/k! computed directly as the product (k+1)*(k+2)*...*n (never forming n! or k! alone), checked throughout.
//! tags: number, lah, partition, list, ordered, combinatorics, set, checked, wide, u32, escalate
//! entry: LahNumber::run
//! limits: returns 0 if k == 0 (and n != 0) or if k > n; escalates (halt 0xFF05, needs_wider_math) if an intermediate product overflows u32 -- this can trigger before the true L(n,k) result would itself exceed u32::MAX
struct LahNumber { n: u32, k: u32, result: u32 }
impl LahNumber {
    fn run(&mut self) -> u16 {
        if self.n == 0u32 && self.k == 0u32 {
            self.result = 1u32;
            return 1u16;
        }
        if self.k == 0u32 || self.k > self.n {
            self.result = 0u32;
            return 1u16;
        }
        let nm1 = self.n - 1u32;
        let km1 = self.k - 1u32;
        let mut kk = km1;
        if nm1 - km1 < km1 { kk = nm1 - km1; }
        let mut c = 1u32;
        let mut i = 1u32;
        while i <= kk {
            let term = nm1 - kk + i;
            let num = mul_checked_u32(c, term);
            c = num / i;
            i = i + 1u32;
        }
        let mut fact_ratio = 1u32;
        let mut j = self.k + 1u32;
        while j <= self.n {
            fact_ratio = mul_checked_u32(fact_ratio, j);
            j = j + 1u32;
        }
        self.result = mul_checked_u32(c, fact_ratio);
        1u16
    }
}
