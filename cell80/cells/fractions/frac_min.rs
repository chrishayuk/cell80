//! The smaller of two fractions na/da and nb/db, by cross-multiplication (works on unreduced fractions) — returns its numerator/denominator as given (ties keep na/da). Distinct from frac_cmp, which only returns an ordering code, not the winning fraction itself.
//! tags: fraction, frac, min, minimum, compare, wide, u32, checked, escalate
//! entry: FracMin::run
//! limits: escalates (halt 0xFF06, out_of_domain) if da == 0 or db == 0; escalates (halt 0xFF05, needs_wider_math) if either cross-product overflows u32
struct FracMin { na: u32, da: u32, nb: u32, db: u32, num: u32, den: u32 }
impl FracMin {
    fn run(&mut self) -> u16 {
        if self.da == 0u32 || self.db == 0u32 { halt(0xFF06u16); }
        let t1 = mul_checked_u32(self.na, self.db);
        let t2 = mul_checked_u32(self.nb, self.da);
        if t2 < t1 {
            self.num = self.nb;
            self.den = self.db;
        } else {
            self.num = self.na;
            self.den = self.da;
        }
        1u16
    }
}
