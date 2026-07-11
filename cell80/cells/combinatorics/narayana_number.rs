//! Narayana number N(n,k): the count of Dyck paths of length 2n with exactly k peaks, computed as C(n,k)*C(n,k-1)/n for 1<=k<=n (summing over k recovers catalan_number(n)) -- catalan_number has no explicit-k sibling, the same generalization rencontres_number already applies to derangement_count; inlines choose_u32's multiplicative running-division loop twice since cells can't call each other.
//! tags: number, narayana, catalan, combinatorics, dyck-path, peak, choose, binomial, counting, checked, wide, u32, escalate
//! entry: NarayanaNumber::run
//! limits: returns 0 if k < 1 or k > n; escalates (halt 0xFF05, needs_wider_math) if either inlined choose loop's intermediate product or the final multiply overflows u32
struct NarayanaNumber { n: u32, k: u32, result: u32 }
impl NarayanaNumber {
    fn run(&mut self) -> u16 {
        if self.k < 1u32 || self.k > self.n {
            self.result = 0u32;
            return 1u16;
        }
        // C(n, k) via the same multiplicative running-division formula choose_u32 uses.
        let mut kk = self.k;
        if self.n - self.k < self.k { kk = self.n - self.k; }
        let mut c1 = 1u32;
        let mut i = 1u32;
        while i <= kk {
            let term = self.n - kk + i;
            let num = mul_checked_u32(c1, term);
            c1 = num / i;
            i = i + 1u32;
        }
        // C(n, k-1) via the same formula again, on the same n.
        let km1 = self.k - 1u32;
        let mut kk2 = km1;
        if self.n - km1 < km1 { kk2 = self.n - km1; }
        let mut c2 = 1u32;
        let mut j = 1u32;
        while j <= kk2 {
            let term = self.n - kk2 + j;
            let num = mul_checked_u32(c2, term);
            c2 = num / j;
            j = j + 1u32;
        }
        let prod = mul_checked_u32(c1, c2);
        self.result = prod / self.n;
        1u16
    }
}
