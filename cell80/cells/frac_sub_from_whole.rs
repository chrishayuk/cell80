//! Subtract a fraction from a whole number: whole - n/d, reduced to lowest terms via the shared gcd_u32 kernel.
//! tags: fraction, frac, subtract, whole, integer, mixed, wide, u32, checked, escalate, remaining, work, job, left
//! entry: FracSubFromWhole::run
//! limits: escalates (halt 0xFF06, out_of_domain) if d == 0; escalates (halt 0xFF05, needs_wider_math) if whole*d overflows u32 or n > whole*d (the result would be negative)
struct FracSubFromWhole { whole: u32, n: u32, d: u32, num: u32, den: u32 }
impl FracSubFromWhole {
    fn run(&mut self) -> u16 {
        if self.d == 0u32 { halt(0xFF06u16); }
        let wd = mul_checked_u32(self.whole, self.d);
        if self.n > wd { halt(0xFF05u16); }
        let num_raw = wd - self.n;
        if num_raw == 0u32 {
            self.num = 0u32;
            self.den = 1u32;
            return 1u16;
        }
        let g = gcd_u32(num_raw, self.d);
        self.num = num_raw / g;
        self.den = self.d / g;
        1u16
    }
}
