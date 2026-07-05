//! Compare two fractions na/da vs nb/db via cross-multiplication (works on unreduced fractions, e.g. 1/2 vs 2/4): 0 if less, 1 if equal, 2 if greater.
//! tags: fraction, frac, compare, cmp, order, ordering, wide, u32, checked
//! entry: FracCmp::run
//! limits: escalates (halt 0xFF06, out_of_domain) if da == 0 or db == 0; escalates (halt 0xFF05, needs_wider_math) if a cross-product overflows u32
struct FracCmp { na: u32, da: u32, nb: u32, db: u32 }
impl FracCmp {
    fn run(&mut self) -> u16 {
        if self.da == 0u32 || self.db == 0u32 { halt(0xFF06u16); }
        let t1 = mul_checked_u32(self.na, self.db);
        let t2 = mul_checked_u32(self.nb, self.da);
        if t1 < t2 {
            0u16
        } else if t1 == t2 {
            1u16
        } else {
            2u16
        }
    }
}
