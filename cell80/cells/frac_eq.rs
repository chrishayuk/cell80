//! Returns 1 if two fractions na/da and nb/db are equal, else 0 — via cross-multiplication, so unreduced-but-equivalent fractions (e.g. 1/2 vs 2/4) still compare equal without needing to reduce first.
//! tags: fraction, frac, equal, equals, compare, wide, u32, checked
//! entry: FracEq::run
//! limits: escalates (halt 0xFF06, out_of_domain) if da == 0 or db == 0; escalates (halt 0xFF05, needs_wider_math) if a cross-product overflows u32
struct FracEq { na: u32, da: u32, nb: u32, db: u32 }
impl FracEq {
    fn run(&mut self) -> u16 {
        if self.da == 0u32 || self.db == 0u32 { halt(0xFF06u16); }
        let t1 = self.na.wrapping_mul(self.db);
        if self.na != 0u32 && t1 / self.na != self.db { halt(0xFF05u16); }
        let t2 = self.nb.wrapping_mul(self.da);
        if self.nb != 0u32 && t2 / self.nb != self.da { halt(0xFF05u16); }
        (t1 == t2) as u16
    }
}
