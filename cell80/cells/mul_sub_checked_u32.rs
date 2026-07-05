//! Checked fused multiply-subtract at u32: a*b-c, escalating if the multiply overflows or c exceeds the product (e.g. a per-unit price times a quantity, minus a flat discount).
//! tags: math, multiply, subtract, fma, checked, wide, u32, overflow, negative, escalate
//! entry: MulSubChecked::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if a * b overflows u32 or c > a*b
struct MulSubChecked { a: u32, b: u32, c: u32, result: u32 }
impl MulSubChecked {
    fn run(&mut self) -> u16 {
        let p = self.a.wrapping_mul(self.b);
        if self.a != 0u32 && p / self.a != self.b { halt(0xFF05u16); }
        if self.c > p { halt(0xFF05u16); }
        self.result = p - self.c;
        1u16
    }
}
