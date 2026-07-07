//! Returns 1 if a fraction n/d is proper (n < d, i.e. less than one whole), else 0. Escalates (halt 0xFF06, out_of_domain) if d == 0.
//! tags: fraction, frac, proper, whole, predicate, wide, u32
//! entry: FracIsProper::run
//! limits: escalates (halt 0xFF06, out_of_domain) if d == 0
struct FracIsProper { n: u32, d: u32, ok: u16 }
impl FracIsProper {
    fn run(&mut self) -> u16 {
        if self.d == 0u32 { halt(0xFF06u16); }
        self.ok = (self.n < self.d) as u16;
        self.ok
    }
}
