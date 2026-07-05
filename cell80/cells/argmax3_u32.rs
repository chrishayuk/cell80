//! Index (0, 1, or 2) of the largest of three values at wide u32 width; ties -> lowest index — the wide sibling of argmax3 (which works over u16 and can't rank values beyond 65535, e.g. money totals in cents).
//! tags: argmax, index, which, largest, choose, select, wide, u32, large
//! entry: Argmax3Wide::run
struct Argmax3Wide { a: u32, b: u32, c: u32 }
impl Argmax3Wide {
    fn run(&mut self) -> u16 {
        if self.b > self.a {
            if self.c > self.b { 2u16 } else { 1u16 }
        } else if self.c > self.a { 2u16 } else { 0u16 }
    }
}
