//! The nth derangement number (D(0)=1, D(1)=0, D(n)=(n-1)*(D(n-1)+D(n-2)) — the count of permutations of n items with no fixed point), checked: escalates instead of silently wrapping once D(n) would exceed u32::MAX (n >= 14). Unlike catalan_number's recurrence, this one's intermediate never overflows before the true result itself would (verified) — the multiplier grows linearly (n-1) against a linearly-combined sum, not against an already-exponential value.
//! tags: number, derangement, combinatorics, permutation, sequence, counting, checked, wide, u32, escalate
//! entry: DerangementCount::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if D(n) would exceed u32::MAX (n >= 14)
struct DerangementCount { n: u32, result: u32 }
impl DerangementCount {
    fn run(&mut self) -> u16 {
        let mut d_prev2 = 1u32;
        let mut d_prev1 = 0u32;
        if self.n == 0u32 {
            self.result = d_prev2;
            return 1u16;
        }
        if self.n == 1u32 {
            self.result = d_prev1;
            return 1u16;
        }
        let mut i = 2u32;
        let mut d = 0u32;
        while i <= self.n {
            let s = add_checked_u32(d_prev1, d_prev2);
            let mult = i - 1u32;
            let prod = mult.wrapping_mul(s);
            if mult != 0u32 && prod / mult != s { halt(0xFF05u16); }
            d = prod;
            d_prev2 = d_prev1;
            d_prev1 = d;
            i = i + 1u32;
        }
        self.result = d;
        1u16
    }
}
