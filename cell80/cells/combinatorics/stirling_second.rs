//! Stirling number of the second kind S(n, k): the number of ways to partition an n-element set into exactly k non-empty subsets. Computed via the inclusion-exclusion closed form S(n,k) = (1/k!) * sum_{j=0}^{k} (-1)^(k-j) * C(k,j) * j^n -- the alternating sum tracked as a sign-magnitude pair (no array needed), C(k,j) via the same multiplicative running-division formula choose_u32 uses, then divided exactly by k! at the end.
//! tags: number, stirling, second-kind, partition, subset, combinatorics, checked, wide, u32, escalate
//! entry: StirlingSecond::run
//! limits: returns 0 if k > n; escalates (halt 0xFF05, needs_wider_math) if an intermediate term overflows u32 -- this can trigger well before the true S(n,k) itself would (the same known limitation choose_u32 documents for its own intermediates)
struct StirlingSecond { n: u32, k: u32, result: u32 }
impl StirlingSecond {
    fn run(&mut self) -> u16 {
        if self.k > self.n {
            self.result = 0u32;
            return 1u16;
        }
        if self.k == 0u32 {
            let base_case = if self.n == 0u32 { 1u32 } else { 0u32 };
            self.result = base_case;
            return 1u16;
        }
        let mut sum_mag = 0u32;
        let mut sum_neg = 0u16;
        let mut j = 0u32;
        while j <= self.k {
            let mut jj = j;
            if self.k - j < j { jj = self.k - j; }
            let mut c = 1u32;
            let mut i = 1u32;
            while i <= jj {
                let term = self.k - jj + i;
                let num = mul_checked_u32(c, term);
                c = num / i;
                i = i + 1u32;
            }
            let mut p = 1u32;
            let mut e = 0u32;
            while e < self.n {
                p = mul_checked_u32(p, j);
                e = e + 1u32;
            }
            let term_mag = mul_checked_u32(c, p);
            let term_neg = if (self.k - j) % 2u32 == 0u32 { 0u16 } else { 1u16 };
            if term_mag != 0u32 {
                if sum_neg == term_neg {
                    sum_mag = add_checked_u32(sum_mag, term_mag);
                } else if sum_mag >= term_mag {
                    sum_mag = sum_mag - term_mag;
                } else {
                    sum_mag = term_mag - sum_mag;
                    sum_neg = term_neg;
                }
            }
            j = j + 1u32;
        }
        let mut kfact = 1u32;
        let mut m = 1u32;
        while m <= self.k {
            kfact = mul_checked_u32(kfact, m);
            m = m + 1u32;
        }
        self.result = sum_mag / kfact;
        1u16
    }
}
