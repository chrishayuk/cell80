//! Verifies a claimed ceiling-division quotient: 0 if b == 0, else q = a / b, r = a % b, rounded = q + 1 if r != 0 else q, and returns 1 if rounded == quotient, else 0 — the verifier counterpart of div_ceil_u32 (that one computes and escalates on b == 0; this one checks a candidate answer and always returns a verdict).
//! tags: verify, verifier, equation, quotient, divide, ceiling, round-up, wide, u32, check, plan, reverse-equation
//! entry: QuotientEqualsCeil::run
struct QuotientEqualsCeil { a: u32, b: u32, quotient: u32 }
impl QuotientEqualsCeil {
    fn run(&mut self) -> u16 {
        if self.b == 0u32 {
            0u16
        } else {
            let q = self.a / self.b;
            let r = self.a % self.b;
            let rounded = if r != 0u32 { q + 1u32 } else { q };
            (rounded == self.quotient) as u16
        }
    }
}
