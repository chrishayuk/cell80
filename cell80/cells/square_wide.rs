//! Exact square with a wide u32 result field: sq = n*n, no u16 cap (the value cell square saturates).
//! tags: math, square, power, multiply, wide, exact
//! entry: Sq::run
struct Sq { n: u16, sq: u32 }
impl Sq {
    fn run(&mut self) -> u16 {
        self.sq = self.n as u32 * self.n as u32;
        if (self.sq >> 16u32) as u16 != 0u16 { 65535u16 } else { self.sq as u16 }
    }
}
