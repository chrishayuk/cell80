//! Subtract two fractions na/da - nb/db, reduced to lowest terms via an inline GCD.
//! tags: fraction, frac, subtract, difference, arithmetic, wide, u32, checked
//! entry: FracSub::run
//! limits: escalates (halt 0xFF06, out_of_domain) if da == 0 or db == 0; escalates (halt 0xFF05, needs_wider_math) if the result would be negative (an unsigned fraction can't represent it) or a cross-product/denominator overflows u32
struct FracSub { na: u32, da: u32, nb: u32, db: u32, num: u32, den: u32 }
impl FracSub {
    fn run(&mut self) -> u16 {
        if self.da == 0u32 || self.db == 0u32 { halt(0xFF06u16); }
        let t1 = self.na.wrapping_mul(self.db);
        if self.na != 0u32 && t1 / self.na != self.db { halt(0xFF05u16); }
        let t2 = self.nb.wrapping_mul(self.da);
        if self.nb != 0u32 && t2 / self.nb != self.da { halt(0xFF05u16); }
        if t1 < t2 { halt(0xFF05u16); }
        let num_raw = t1 - t2;
        let den_raw = self.da.wrapping_mul(self.db);
        if self.da != 0u32 && den_raw / self.da != self.db { halt(0xFF05u16); }
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
