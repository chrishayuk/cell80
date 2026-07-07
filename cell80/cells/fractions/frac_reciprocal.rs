//! Reciprocal of a fraction n/d: swaps to d/n. Escalates (halt 0xFF06, out_of_domain) if n == 0 (a zero fraction has no reciprocal) or d == 0 (not a valid fraction to begin with).
//! tags: fraction, frac, reciprocal, invert, wide, u32
//! entry: FracReciprocal::run
//! limits: escalates (halt 0xFF06, out_of_domain) if n == 0 or d == 0
struct FracReciprocal { n: u32, d: u32, num: u32, den: u32 }
impl FracReciprocal {
    fn run(&mut self) -> u16 {
        if self.d == 0u32 || self.n == 0u32 { halt(0xFF06u16); }
        self.num = self.d;
        self.den = self.n;
        1u16
    }
}
