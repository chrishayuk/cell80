//! The nth term of an arithmetic sequence starting at start with common difference step: start + step*(n-1), 1-indexed (n=1 is the first term) — the missing nth-term sibling of arithmetic_series_sum (which only sums the sequence, not a single term).
//! tags: number, arithmetic, sequence, nth, term, math, checked, wide, u32, escalate
//! entry: ArithmeticNthWide::run
//! limits: escalates (halt 0xFF06, out_of_domain) if n == 0; escalates (halt 0xFF05, needs_wider_math) if the multiply or add overflows
struct ArithmeticNthWide { start: u32, step: u32, n: u32, result: u32 }
impl ArithmeticNthWide {
    fn run(&mut self) -> u16 {
        if self.n == 0u32 { halt(0xFF06u16); }
        let nm1 = self.n - 1u32;
        let term = mul_checked_u32(self.step, nm1);
        self.result = add_checked_u32(self.start, term);
        1u16
    }
}
