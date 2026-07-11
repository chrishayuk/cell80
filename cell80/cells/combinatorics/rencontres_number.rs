//! D(n,k): the count of permutations of n elements with exactly k fixed points, via D(n,k) = C(n,k) * D(n-k) — generalizes derangement_count (its own implicit k=0 case) with an explicit fixed-point count, the same way polygonal_number generalized triangular with an explicit side count; inlines the same choose-style multiplicative running-division loop choose_u32 uses and the same D(m)=(m-1)*(D(m-1)+D(m-2)) recurrence derangement_count uses, since cells can't call each other.
//! tags: number, rencontres, derangement, combinatorics, permutation, fixed-point, choose, binomial, sequence, counting, checked, wide, u32, escalate
//! entry: RencontresNumber::run
//! limits: returns 0 if k > n; escalates (halt 0xFF05, needs_wider_math) if an intermediate choose product, the derangement recurrence's product, or the final choose*derangement multiply overflows u32
struct RencontresNumber { n: u32, k: u32, result: u32 }
impl RencontresNumber {
    fn run(&mut self) -> u16 {
        if self.k > self.n {
            self.result = 0u32;
            return 1u16;
        }
        // C(n,k) via the same multiplicative running-division formula choose_u32 uses.
        let mut kk = self.k;
        if self.n - self.k < self.k { kk = self.n - self.k; }
        let mut c = 1u32;
        let mut i = 1u32;
        while i <= kk {
            let term = self.n - kk + i;
            let num = mul_checked_u32(c, term);
            c = num / i;
            i = i + 1u32;
        }
        // D(n-k) via the same (m-1)*(D(m-1)+D(m-2)) recurrence derangement_count uses.
        let m = self.n - self.k;
        let mut d_prev2 = 1u32;
        let mut d_prev1 = 0u32;
        let mut d = d_prev2;
        if m == 0u32 {
            d = d_prev2;
        } else if m == 1u32 {
            d = d_prev1;
        } else {
            let mut j = 2u32;
            while j <= m {
                let s = add_checked_u32(d_prev1, d_prev2);
                let mult = j - 1u32;
                let prod = mult.wrapping_mul(s);
                if mult != 0u32 && prod / mult != s { halt(0xFF05u16); }
                d = prod;
                d_prev2 = d_prev1;
                d_prev1 = d;
                j = j + 1u32;
            }
        }
        self.result = mul_checked_u32(c, d);
        1u16
    }
}
