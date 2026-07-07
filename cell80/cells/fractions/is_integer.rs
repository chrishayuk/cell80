//! Returns 1 if the wide fraction n/d is a whole number (n divides evenly by d), else 0 — a wrong-plan signal for word problems that expect an exact split.
//! tags: fraction, frac, integer, whole, exact, divide, wide, u32, checked
//! entry: IsInteger::run
//! limits: escalates (halt 0xFF06, out_of_domain) if d == 0
struct IsInteger { n: u32, d: u32 }
impl IsInteger {
    fn run(&mut self) -> u16 {
        if self.d == 0u32 { halt(0xFF06u16); }
        (self.n % self.d == 0u32) as u16
    }
}
