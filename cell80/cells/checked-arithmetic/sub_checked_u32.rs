//! Checked u32 subtract: escalates (needs_wider_math) instead of wrapping if b > a (the result would be negative).
//! tags: math, subtract, checked, wide, u32, negative, escalate, rate, net-rate
//! entry: SubChecked::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if b > a
struct SubChecked { a: u32, b: u32, diff: u32 }
impl SubChecked {
    fn run(&mut self) -> u16 {
        if self.b > self.a { halt(0xFF05u16); }
        self.diff = self.a - self.b;
        1u16
    }
}
