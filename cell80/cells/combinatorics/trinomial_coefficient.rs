//! Trinomial coefficient n!/(k1!*k2!*k3!) with k3 = n-k1-k2 (the number of ways to split n labeled items into 3 labeled groups of sizes k1, k2, k3), computed as choose(n,k1)*choose(n-k1,k2) -- choose_u32 has no 3-way sibling despite this pack repeatedly generalizing 2-parameter cells with one explicit extra parameter (rencontres_number, and polygonal_number/divisor_power_sum elsewhere); inlines choose_u32's multiplicative running-division loop twice since cells can't call each other.
//! tags: math, combinatorics, trinomial, multinomial, choose, binomial, combination, counting, checked, wide, u32, escalate
//! entry: TrinomialCoeff::run
//! limits: returns 0 if k1 + k2 > n; escalates (halt 0xFF05, needs_wider_math) if either inlined choose loop's intermediate product or the final multiply overflows u32
struct TrinomialCoeff { n: u32, k1: u32, k2: u32, result: u32 }
impl TrinomialCoeff {
    fn run(&mut self) -> u16 {
        if self.k1 + self.k2 > self.n {
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
        let rem = self.n - self.k1;
        let mut kk2 = self.k2;
        if rem - self.k2 < self.k2 { kk2 = rem - self.k2; }
        let mut c2 = 1u32;
        let mut j = 1u32;
        while j <= kk2 {
            let term = rem - kk2 + j;
            let num = mul_checked_u32(c2, term);
            c2 = num / j;
            j = j + 1u32;
        }
        self.result = mul_checked_u32(c1, c2);
        1u16
    }
}
