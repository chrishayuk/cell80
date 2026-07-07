//! Exact weighted sum with a wide u32 result field: sum = a + 2b + 3c, no u16 wrap (sibling of weighted_sum).
//! tags: scoring, score, math, combine, weighted, wide, exact
//! entry: Ws::run
struct Ws { a: u16, b: u16, c: u16, sum: u32 }
impl Ws {
    fn run(&mut self) -> u16 {
        self.sum = self.a as u32 + self.b as u32 * 2u32 + self.c as u32 * 3u32;
        if (self.sum >> 16u32) as u16 != 0u16 { 65535u16 } else { self.sum as u16 }
    }
}
