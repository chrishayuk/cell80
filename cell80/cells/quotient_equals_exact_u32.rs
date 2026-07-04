//! Verifies a claimed exact wide quotient: 1 if b != 0, a divides evenly by b (a % b == 0), and a / b == quotient, else 0 — the verifier counterpart of div_exact_u32 (that one computes and escalates on a remainder; this one checks a candidate answer and always returns a verdict).
//! tags: verify, verifier, equation, quotient, divide, exact, wide, u32, check, plan, reverse-equation
//! entry: QuotientEqualsExact::run
struct QuotientEqualsExact { a: u32, b: u32, quotient: u32 }
impl QuotientEqualsExact {
    fn run(&mut self) -> u16 {
        if self.b == 0u32 {
            0u16
        } else {
            let q = self.a / self.b;
            let rem = self.a % self.b;
            (rem == 0u32 && q == self.quotient) as u16
        }
    }
}
