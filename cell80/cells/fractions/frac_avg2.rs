//! Average of two fractions na/da and nb/db, reduced to lowest terms via the shared gcd_u32 kernel.
//! tags: fraction, frac, average, mean, midpoint, wide, u32, checked, escalate
//! entry: FracAvg2::run
//! limits: escalates (halt 0xFF06, out_of_domain) if da == 0 or db == 0; escalates (halt 0xFF05, needs_wider_math) if any cross-product, the combined numerator, or the combined denominator overflows u32
struct FracAvg2 { na: u32, da: u32, nb: u32, db: u32, num: u32, den: u32 }
impl FracAvg2 {
    fn run(&mut self) -> u16 {
        if self.da == 0u32 || self.db == 0u32 { halt(0xFF06u16); }
        let t1 = mul_checked_u32(self.na, self.db);
        let t2 = mul_checked_u32(self.nb, self.da);
        let num_raw = add_checked_u32(t1, t2);
        let dd = mul_checked_u32(self.da, self.db);
        let den_raw = dd.wrapping_mul(2u32);
        if den_raw < dd { halt(0xFF05u16); }
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
