//! Verifies a claimed wide fused multiply-subtract: returns 1 if a * b - c == total, else 0, including when the multiply overflows u32 or c exceeds the product — the reverse-equation counterpart of mul_sub_checked_u32.
//! tags: verify, verifier, equation, multiply, subtract, fma, wide, u32, check, plan, reverse-equation
//! entry: MulSubEqualsWide::run
struct MulSubEqualsWide { a: u32, b: u32, c: u32, total: u32 }
impl MulSubEqualsWide {
    fn run(&mut self) -> u16 {
        let p = self.a.wrapping_mul(self.b);
        if self.a != 0u32 && p / self.a != self.b {
            0u16
        } else if self.c > p {
            0u16
        } else {
            ((p - self.c) == self.total) as u16
        }
    }
}
