//! Exact u32 division: escalates (needs_wider_math) if b is zero or a doesn't divide evenly by b — a wrong-plan signal for word problems that declared an exact division.
//! tags: math, divide, exact, checked, wide, u32, remainder, escalate, rate, time, unit-rate, proportion
//! entry: DivExact::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if b == 0 or a % b != 0
struct DivExact { a: u32, b: u32, quotient: u32 }
impl DivExact {
    fn run(&mut self) -> u16 {
        if self.b == 0u32 { halt(0xFF05u16); }
        if self.a % self.b != 0u32 { halt(0xFF05u16); }
        self.quotient = self.a / self.b;
        1u16
    }
}
