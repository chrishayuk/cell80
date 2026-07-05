//! Checked u32 add: escalates (needs_wider_math) instead of silently wrapping if a + b overflows u32.
//! tags: math, add, checked, wide, u32, overflow, escalate
//! entry: AddChecked::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if a + b would exceed u32::MAX
struct AddChecked { a: u32, b: u32, sum: u32 }
impl AddChecked {
    fn run(&mut self) -> u16 {
        let s = add_checked_u32(self.a, self.b);
        self.sum = s;
        1u16
    }
}
