//! Verifies a claimed wide fused multiply-add: returns 1 if a * b + c == total, else 0, including when either step overflows u32 — the reverse-equation counterpart of mul_add_checked_u32.
//! tags: verify, verifier, equation, multiply, add, fma, wide, u32, check, plan, reverse-equation
//! entry: MulAddEqualsWide::run
struct MulAddEqualsWide { a: u32, b: u32, c: u32, total: u32 }
impl MulAddEqualsWide {
    fn run(&mut self) -> u16 {
        let p = self.a.wrapping_mul(self.b);
        if self.a != 0u32 && p / self.a != self.b {
            0u16
        } else {
            let s = p.wrapping_add(self.c);
            if s < p { 0u16 } else { (s == self.total) as u16 }
        }
    }
}
