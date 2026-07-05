//! Maximum of two wide u32 values — the exact wide sibling of max (which works over u16).
//! tags: math, max, maximum, larger, bigger, greatest, greater, compare, select, wide, u32, large
//! entry: MaxWide::run
struct MaxWide { a: u32, b: u32, result: u32 }
impl MaxWide {
    fn run(&mut self) -> u16 {
        let m = if self.a > self.b { self.a } else { self.b };
        self.result = m;
        1u16
    }
}
