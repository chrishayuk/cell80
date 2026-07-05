//! Minimum of two wide u32 values — the exact wide sibling of min (which works over u16).
//! tags: math, min, minimum, smaller, smallest, least, lesser, compare, select, wide, u32, large
//! entry: MinWide::run
struct MinWide { a: u32, b: u32, result: u32 }
impl MinWide {
    fn run(&mut self) -> u16 {
        let m = if self.a < self.b { self.a } else { self.b };
        self.result = m;
        1u16
    }
}
