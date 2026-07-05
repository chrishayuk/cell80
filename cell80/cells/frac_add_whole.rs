//! Add a whole number to a fraction: n/d + whole = (n + whole*d)/d, reduced to lowest terms via the shared gcd_u32 kernel.
//! tags: fraction, frac, add, whole, integer, mixed, wide, u32, checked, escalate
//! entry: FracAddWhole::run
//! limits: escalates (halt 0xFF06, out_of_domain) if d == 0; escalates (halt 0xFF05, needs_wider_math) if whole*d or the final sum overflows u32
struct FracAddWhole { n: u32, d: u32, whole: u32, num: u32, den: u32 }
impl FracAddWhole {
    fn run(&mut self) -> u16 {
        if self.d == 0u32 { halt(0xFF06u16); }
        let wd = self.whole.wrapping_mul(self.d);
        if self.whole != 0u32 && wd / self.whole != self.d { halt(0xFF05u16); }
        let num_raw = self.n.wrapping_add(wd);
        if num_raw < self.n { halt(0xFF05u16); }
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
