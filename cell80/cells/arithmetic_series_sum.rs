//! Sum of the first n terms of an arithmetic sequence starting at a with common difference d: n*(2a + (n-1)*d) / 2 — always an exact integer (the product n*(2a+(n-1)*d) is provably always even), checked for overflow at each step.
//! tags: number, arithmetic, series, sequence, sum, math, checked, wide, u32, escalate
//! entry: ArithmeticSeriesSum::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if any intermediate product or sum overflows u32
struct ArithmeticSeriesSum { a: u32, d: u32, n: u32, result: u32 }
impl ArithmeticSeriesSum {
    fn run(&mut self) -> u16 {
        if self.n == 0u32 {
            self.result = 0u32;
            return 1u16;
        }
        let nm1 = self.n - 1u32;
        let dm = nm1.wrapping_mul(self.d);
        if nm1 != 0u32 && dm / nm1 != self.d { halt(0xFF05u16); }
        let two_a = self.a.wrapping_mul(2u32);
        if two_a < self.a { halt(0xFF05u16); }
        let inner = add_checked_u32(two_a, dm);
        let prod = self.n.wrapping_mul(inner);
        if self.n != 0u32 && prod / self.n != inner { halt(0xFF05u16); }
        self.result = prod / 2u32;
        1u16
    }
}
