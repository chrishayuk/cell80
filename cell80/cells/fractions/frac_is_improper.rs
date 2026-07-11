//! Returns 1 if a fraction n/d is improper (n >= d, i.e. one whole or more), else 0 — the explicit complement of frac_is_proper. Escalates (halt 0xFF06, out_of_domain) if d == 0.
//! tags: fraction, frac, improper, whole, predicate, wide, u32
//! entry: FracIsImproper::run
//! limits: escalates (halt 0xFF06, out_of_domain) if d == 0
struct FracIsImproper { n: u32, d: u32, ok: u16 }
impl FracIsImproper {
    fn run(&mut self) -> u16 {
        if self.d == 0u32 { halt(0xFF06u16); }
        self.ok = (self.n >= self.d) as u16;
        self.ok
    }
}
