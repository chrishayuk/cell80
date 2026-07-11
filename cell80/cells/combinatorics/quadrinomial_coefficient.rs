//! Quadrinomial coefficient n!/(k1!*k2!*k3!*k4!) with k4 = n-k1-k2-k3 (the number of ways to split n labeled items into 4 labeled groups of sizes k1, k2, k3, k4), computed as choose(n,k1)*choose(n-k1,k2)*choose(n-k1-k2,k3) -- the direct arity-4 extension of trinomial_coefficient's arity-3 formula, flagged by that cell's own summary noting choose_u32 has no 3-way sibling; inlines choose_u32's multiplicative running-division loop three times since cells can't call each other.
//! tags: math, combinatorics, quadrinomial, multinomial, trinomial, choose, binomial, combination, counting, checked, wide, u32, escalate
//! entry: QuadrinomialCoeff::run
//! limits: returns 0 if k1 + k2 + k3 > n; escalates (halt 0xFF05, needs_wider_math) if any inlined choose loop's intermediate product or the final multiplies overflow u32
struct QuadrinomialCoeff { n: u32, k1: u32, k2: u32, k3: u32, result: u32 }
impl QuadrinomialCoeff {
    fn run(&mut self) -> u16 {
        if self.k1 + self.k2 + self.k3 > self.n {
            self.result = 0u32;
            return 1u16;
        }
        // C(n, k1) via the same multiplicative running-division formula choose_u32 uses.
        let mut kk1 = self.k1;
        if self.n - self.k1 < self.k1 { kk1 = self.n - self.k1; }
        let mut c1 = 1u32;
        let mut i = 1u32;
        while i <= kk1 {
            let term = self.n - kk1 + i;
            let num = mul_checked_u32(c1, term);
            c1 = num / i;
            i = i + 1u32;
        }
        // C(n - k1, k2) via the same formula again, on the remaining pool.
        let rem1 = self.n - self.k1;
        let mut kk2 = self.k2;
        if rem1 - self.k2 < self.k2 { kk2 = rem1 - self.k2; }
        let mut c2 = 1u32;
        let mut j = 1u32;
        while j <= kk2 {
            let term = rem1 - kk2 + j;
            let num = mul_checked_u32(c2, term);
            c2 = num / j;
            j = j + 1u32;
        }
        // C(n - k1 - k2, k3) via the same formula again, on the further-reduced pool.
        let rem2 = rem1 - self.k2;
        let mut kk3 = self.k3;
        if rem2 - self.k3 < self.k3 { kk3 = rem2 - self.k3; }
        let mut c3 = 1u32;
        let mut m = 1u32;
        while m <= kk3 {
            let term = rem2 - kk3 + m;
            let num = mul_checked_u32(c3, term);
            c3 = num / m;
            m = m + 1u32;
        }
        let c12 = mul_checked_u32(c1, c2);
        self.result = mul_checked_u32(c12, c3);
        1u16
    }
}
