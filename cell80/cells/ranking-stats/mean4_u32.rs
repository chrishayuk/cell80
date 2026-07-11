//! Mean (average) of four wide u32 values, written to a result field via mean4's div/remainder-recombine trick generalized to u32 — the wide sibling of mean4 (which works over u16).
//! tags: mean, average, avg, four, stat, central, wide, u32, large
//! entry: Mean4Wide::run
struct Mean4Wide { a: u32, b: u32, c: u32, d: u32, result: u32 }
impl Mean4Wide {
    fn run(&mut self) -> u16 {
        self.result = self.a / 4u32 + self.b / 4u32 + self.c / 4u32 + self.d / 4u32
            + (self.a % 4u32 + self.b % 4u32 + self.c % 4u32 + self.d % 4u32) / 4u32;
        1u16
    }
}
