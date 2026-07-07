//! A fraction of a whole number, computed exactly: n/d * whole, escalating if it doesn't divide evenly (a wrong-plan signal — e.g. "3/4 of 20" should be exact for a grade-school word problem) or if the multiply overflows.
//! tags: fraction, frac, of, whole, multiply, exact, wide, u32, checked, escalate, scale, unit, convert, conversion, dollars, cents, hours, minutes, percent, ratio, recipe
//! entry: FracOfWhole::run
//! limits: escalates (halt 0xFF06, out_of_domain) if d == 0 or the product isn't evenly divisible by d; escalates (halt 0xFF05, needs_wider_math) if n * whole overflows u32
struct FracOfWhole { n: u32, d: u32, whole: u32, result: u32 }
impl FracOfWhole {
    fn run(&mut self) -> u16 {
        if self.d == 0u32 { halt(0xFF06u16); }
        let p = mul_checked_u32(self.n, self.whole);
        if p % self.d != 0u32 { halt(0xFF06u16); }
        self.result = p / self.d;
        1u16
    }
}
