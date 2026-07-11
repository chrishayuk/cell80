//! Divide a fraction by a whole number, staying a fraction: (n/d) / k = n/(d*k), reduced to lowest terms via the shared gcd_u32 kernel — the missing divide-direction sibling of frac_scale (which multiplies by a whole).
//! tags: fraction, frac, divide, whole, integer, reduce, wide, u32, checked, escalate
//! entry: FracDivWhole::run
//! limits: escalates (halt 0xFF06, out_of_domain) if d == 0 or k == 0; escalates (halt 0xFF05, needs_wider_math) if d * k overflows u32
struct FracDivWhole { n: u32, d: u32, k: u32, num: u32, den: u32 }
impl FracDivWhole {
    fn run(&mut self) -> u16 {
        if self.d == 0u32 || self.k == 0u32 { halt(0xFF06u16); }
        let den_raw = mul_checked_u32(self.d, self.k);
        if self.n == 0u32 {
            self.num = 0u32;
            self.den = 1u32;
            return 1u16;
        }
        let g = gcd_u32(self.n, den_raw);
        self.num = self.n / g;
        self.den = den_raw / g;
        1u16
    }
}
