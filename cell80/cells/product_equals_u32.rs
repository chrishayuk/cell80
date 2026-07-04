//! Verifies a claimed wide product: 1 if a * b == total, else 0 — including when a * b overflows u32 (a real overflow just means the claim doesn't hold, not an escalation; a verifier always returns a verdict).
//! tags: verify, verifier, equation, product, multiply, wide, u32, check, plan, reverse-equation
//! entry: ProductEquals::run
struct ProductEquals { a: u32, b: u32, total: u32 }
impl ProductEquals {
    fn run(&mut self) -> u16 {
        let product = self.a.wrapping_mul(self.b);
        if self.a != 0u32 && product / self.a != self.b {
            0u16
        } else {
            (product == self.total) as u16
        }
    }
}
