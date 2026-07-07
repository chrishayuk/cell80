//! Verifies a claimed wide sum: returns 1 if a + b == total, else 0, without escalating on overflow (a real overflow just means the claim doesn't hold) — the wide sibling of sum_equals (which works over u16).
//! tags: verify, verifier, equation, sum, addition, wide, u32, check, plan, reverse-equation
//! entry: SumEqualsWide::run
struct SumEqualsWide { a: u32, b: u32, total: u32 }
impl SumEqualsWide {
    fn run(&mut self) -> u16 {
        let s = self.a.wrapping_add(self.b);
        if s < self.a {
            0u16
        } else {
            (s == self.total) as u16
        }
    }
}
