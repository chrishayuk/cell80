//! Checked fused multiply-add at u32: a*b+c, escalating on either the multiply or the add overflowing (e.g. a per-unit price times a quantity, plus a flat fee).
//! tags: math, multiply, add, fma, checked, wide, u32, overflow, escalate
//! entry: MulAddChecked::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if a * b or (a*b) + c would exceed u32::MAX
struct MulAddChecked { a: u32, b: u32, c: u32, result: u32 }
impl MulAddChecked {
    fn run(&mut self) -> u16 {
        let p = self.a.wrapping_mul(self.b);
        if self.a != 0u32 && p / self.a != self.b { halt(0xFF05u16); }
        let s = p.wrapping_add(self.c);
        if s < p { halt(0xFF05u16); }
        self.result = s;
        1u16
    }
}
