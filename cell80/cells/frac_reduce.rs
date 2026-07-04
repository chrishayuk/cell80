//! Reduce a fraction n/d to lowest terms via an inline Euclidean GCD (no shared gcd_u32 helper — a two-u32-param function still can't cross a call boundary, so the loop is duplicated in every fraction cell that needs it).
//! tags: fraction, frac, reduce, lowest-terms, gcd, wide, u32, checked
//! entry: FracReduce::run
//! limits: escalates (halt 0xFF06, out_of_domain) if d == 0
struct FracReduce { n: u32, d: u32, num: u32, den: u32 }
impl FracReduce {
    fn run(&mut self) -> u16 {
        if self.d == 0u32 { halt(0xFF06u16); }
        let mut x = self.n;
        let mut y = self.d;
        while y != 0u32 {
            let t = y;
            y = x % y;
            x = t;
        }
        self.num = self.n / x;
        self.den = self.d / x;
        1u16
    }
}
