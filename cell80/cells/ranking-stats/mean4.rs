//! Mean (average) of four values, extending mean3's div/remainder-recombine trick one operand deeper to avoid overflow.
//! tags: mean, average, avg, four, stat, central
//! entry: Mean4::run
struct Mean4 { a: u16, b: u16, c: u16, d: u16 }
impl Mean4 {
    fn run(&mut self) -> u16 {
        self.a / 4u16 + self.b / 4u16 + self.c / 4u16 + self.d / 4u16
            + (self.a % 4u16 + self.b % 4u16 + self.c % 4u16 + self.d % 4u16) / 4u16
    }
}
