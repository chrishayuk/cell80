//! Average of two fractions na/da and nb/db, reduced to lowest terms via an inline GCD.
//! tags: fraction, frac, average, mean, midpoint, wide, u32, checked, escalate
//! entry: FracAvg2::run
//! limits: escalates (halt 0xFF06, out_of_domain) if da == 0 or db == 0; escalates (halt 0xFF05, needs_wider_math) if any cross-product, the combined numerator, or the combined denominator overflows u32
struct FracAvg2 { na: u32, da: u32, nb: u32, db: u32, num: u32, den: u32 }
impl FracAvg2 {
    fn run(&mut self) -> u16 {
        if self.da == 0u32 || self.db == 0u32 { halt(0xFF06u16); }
        let t1 = self.na.wrapping_mul(self.db);
        if self.na != 0u32 && t1 / self.na != self.db { halt(0xFF05u16); }
        let t2 = self.nb.wrapping_mul(self.da);
        if self.nb != 0u32 && t2 / self.nb != self.da { halt(0xFF05u16); }
        let num_raw = t1.wrapping_add(t2);
        if num_raw < t1 { halt(0xFF05u16); }
        let dd = self.da.wrapping_mul(self.db);
        if self.da != 0u32 && dd / self.da != self.db { halt(0xFF05u16); }
        let den_raw = dd.wrapping_mul(2u32);
        if den_raw < dd { halt(0xFF05u16); }
        if num_raw == 0u32 {
            self.num = 0u32;
            self.den = 1u32;
            return 1u16;
        }
        let mut x = num_raw;
        let mut y = den_raw;
        while y != 0u32 {
            let t = y;
            y = x % y;
            x = t;
        }
        self.num = num_raw / x;
        self.den = den_raw / x;
        1u16
    }
}
