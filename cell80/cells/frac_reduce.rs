//! Reduce a fraction n/d to lowest terms via the shared gcd_u32 kernel — a two-u32-param call (first arg rides HL:DE, second rides the stack; docs 10 §Calls), so the Euclidean loop lives once in the prelude instead of inlined in every fraction cell.
//! tags: fraction, frac, reduce, lowest-terms, gcd, wide, u32, checked
//! entry: FracReduce::run
//! limits: escalates (halt 0xFF06, out_of_domain) if d == 0
struct FracReduce { n: u32, d: u32, num: u32, den: u32 }
impl FracReduce {
    fn run(&mut self) -> u16 {
        if self.d == 0u32 { halt(0xFF06u16); }
        let g = gcd_u32(self.n, self.d);
        self.num = self.n / g;
        self.den = self.d / g;
        1u16
    }
}
