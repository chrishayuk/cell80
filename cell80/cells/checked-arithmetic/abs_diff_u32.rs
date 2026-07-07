//! Absolute difference |a - b| between two wide u32 values — the exact wide sibling of abs_diff (which works over u16 and can't represent differences beyond 65535).
//! tags: math, distance, diff, difference, absolute, gap, wide, u32, large
//! entry: AbsDiffWide::run
struct AbsDiffWide { a: u32, b: u32, diff: u32 }
impl AbsDiffWide {
    fn run(&mut self) -> u16 {
        let d = if self.a > self.b { self.a - self.b } else { self.b - self.a };
        self.diff = d;
        1u16
    }
}
