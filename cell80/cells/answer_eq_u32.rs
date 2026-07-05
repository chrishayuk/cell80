//! Verifies a claimed wide answer: returns 1 if a == b, else 0 — the wide sibling of eq (which works over u16 and can't compare values beyond 65535, e.g. money totals in cents).
//! tags: verify, verifier, equation, equal, answer, wide, u32, check, plan
//! entry: AnswerEqWide::run
struct AnswerEqWide { a: u32, b: u32, ok: u16 }
impl AnswerEqWide {
    fn run(&mut self) -> u16 {
        self.ok = (self.a == self.b) as u16;
        self.ok
    }
}
