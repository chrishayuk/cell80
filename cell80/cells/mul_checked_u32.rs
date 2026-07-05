//! Checked u32 multiply: escalates (needs_wider_math) instead of wrapping if a * b overflows u32.
//! tags: math, multiply, checked, wide, u32, overflow, escalate, rate, time, total
//! entry: MulChecked::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if a * b would exceed u32::MAX
struct MulChecked { a: u32, b: u32, product: u32 }
impl MulChecked {
    fn run(&mut self) -> u16 {
        let p = mul_checked_u32(self.a, self.b);
        self.product = p;
        1u16
    }
}
