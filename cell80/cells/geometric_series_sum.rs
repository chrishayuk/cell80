//! Sum of the first n terms of a geometric sequence starting at a with ratio r (a + a*r + a*r^2 + ... + a*r^(n-1)), computed by direct iterative summation rather than the a*(r^n-1)/(r-1) closed form — r^n alone would overflow long before a genuinely unrepresentable sum does, so this escalates exactly when the true sum (or an intermediate term) doesn't fit u32, no earlier. Exact for any r >= 0, not just r > 1.
//! tags: number, geometric, series, sequence, sum, ratio, math, checked, wide, u32, escalate
//! entry: GeometricSeriesSum::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if any term or the running sum overflows u32
struct GeometricSeriesSum { a: u32, r: u32, n: u32, result: u32 }
impl GeometricSeriesSum {
    fn run(&mut self) -> u16 {
        if self.n == 0u32 {
            self.result = 0u32;
            return 1u16;
        }
        let mut term = self.a;
        let mut sum = term;
        let mut i = 1u32;
        while i < self.n {
            let next_term = term.wrapping_mul(self.r);
            if term != 0u32 && next_term / term != self.r { halt(0xFF05u16); }
            term = next_term;
            let next_sum = add_checked_u32(sum, term);
            sum = next_sum;
            i = i + 1u32;
        }
        self.result = sum;
        1u16
    }
}
