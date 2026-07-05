//! Verifies a claimed wide three-way product: returns 1 if a * b * c == total, else 0, including when the product overflows u32 (a real overflow just means the claim doesn't hold) — the reverse-equation counterpart of mul3_checked_u32.
//! tags: verify, verifier, equation, product, multiply, three, wide, u32, check, plan, reverse-equation
//! entry: Product3EqualsWide::run
struct Product3EqualsWide { a: u32, b: u32, c: u32, total: u32 }
impl Product3EqualsWide {
    fn run(&mut self) -> u16 {
        let p1 = self.a.wrapping_mul(self.b);
        if self.a != 0u32 && p1 / self.a != self.b {
            0u16
        } else {
            let p2 = p1.wrapping_mul(self.c);
            if p1 != 0u32 && p2 / p1 != self.c { 0u16 } else { (p2 == self.total) as u16 }
        }
    }
}
