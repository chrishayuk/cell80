//! Scale a fraction by an integer: (n/d) * k, reduced to lowest terms via the shared gcd_u32 kernel — unlike frac_of_whole (which requires an exact whole-number result), this always stays a fraction.
//! tags: fraction, frac, scale, multiply, integer, reduce, wide, u32, checked, escalate
//! entry: FracScale::run
//! limits: escalates (halt 0xFF06, out_of_domain) if d == 0; escalates (halt 0xFF05, needs_wider_math) if n * k overflows u32
struct FracScale { n: u32, d: u32, k: u32, num: u32, den: u32 }
impl FracScale {
    fn run(&mut self) -> u16 {
        if self.d == 0u32 { halt(0xFF06u16); }
        let p = mul_checked_u32(self.n, self.k);
        if p == 0u32 {
            self.num = 0u32;
            self.den = 1u32;
            return 1u16;
        }
        let g = gcd_u32(p, self.d);
        self.num = p / g;
        self.den = self.d / g;
        1u16
    }
}
