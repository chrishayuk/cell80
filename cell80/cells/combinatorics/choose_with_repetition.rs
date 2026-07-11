//! Combinations with repetition allowed (the "multiset coefficient"): the number of ways to choose k items from n types when repeats are permitted, C(n+k-1, k) -- forms n+k-1 with a checked add and reuses choose_u32's own multiplicative running-division formula on (n+k-1, k), unlike choose_u32 itself which explicitly counts subsets *without* repetition.
//! tags: math, combinatorics, choose, binomial, multiset, repetition, stars-and-bars, combination, counting, checked, wide, u32, escalate
//! entry: ChooseWithRepetition::run
//! limits: returns 1 if n = 0 and k = 0, or 0 if n = 0 and k > 0; escalates (halt 0xFF05, needs_wider_math) if n+k-1 overflows u32 or if an intermediate product in the running-division loop overflows u32
struct ChooseWithRepetition { n: u32, k: u32, result: u32 }
impl ChooseWithRepetition {
    fn run(&mut self) -> u16 {
        if self.n == 0u32 {
            let v = if self.k == 0u32 { 1u32 } else { 0u32 };
            self.result = v;
            return 1u16;
        }
        // C(n+k-1, k) via the same multiplicative running-division formula choose_u32 uses.
        let nk = add_checked_u32(self.n, self.k) - 1u32;
        let mut kk = self.k;
        if nk - self.k < self.k { kk = nk - self.k; }
        let mut r = 1u32;
        let mut i = 1u32;
        while i <= kk {
            let term = nk - kk + i;
            let num = mul_checked_u32(r, term);
            r = num / i;
            i = i + 1u32;
        }
        self.result = r;
        1u16
    }
}
