//! Average of two wide u32 values, (a + b) / 2, computed without overflow — the wide sibling of avg2 (which works over u16).
//! tags: math, average, mean, midpoint, halfway, wide, u32, large
//! entry: Avg2Wide::run
struct Avg2Wide { a: u32, b: u32, result: u32 }
impl Avg2Wide {
    fn run(&mut self) -> u16 {
        self.result = (self.a & self.b) + ((self.a ^ self.b) >> 1u32);
        1u16
    }
}
