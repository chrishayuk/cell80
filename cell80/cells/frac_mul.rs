//! Multiply two fractions na/da * nb/db, reduced to lowest terms via the shared gcd_u32 kernel.
//! tags: fraction, frac, multiply, product, arithmetic, wide, u32, checked
//! entry: FracMul::run
//! limits: escalates (halt 0xFF06, out_of_domain) if da == 0 or db == 0; escalates (halt 0xFF05, needs_wider_math) if the numerator or denominator product overflows u32
struct FracMul { na: u32, da: u32, nb: u32, db: u32, num: u32, den: u32 }
impl FracMul {
    fn run(&mut self) -> u16 {
        if self.da == 0u32 || self.db == 0u32 { halt(0xFF06u16); }
        let num_raw = mul_checked_u32(self.na, self.nb);
        let den_raw = mul_checked_u32(self.da, self.db);
        if num_raw == 0u32 {
            self.num = 0u32;
            self.den = 1u32;
            return 1u16;
        }
        let g = gcd_u32(num_raw, den_raw);
        self.num = num_raw / g;
        self.den = den_raw / g;
        1u16
    }
}
