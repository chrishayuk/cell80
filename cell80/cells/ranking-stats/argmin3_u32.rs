//! Index (0, 1, or 2) of the smallest of three values at wide u32 width; ties -> lowest index — the wide sibling of argmin3 (which works over u16 and can't rank values beyond 65535, e.g. money totals in cents).
//! tags: argmin, index, which, smallest, choose, select, wide, u32, large
//! entry: Argmin3Wide::run
struct Argmin3Wide { a: u32, b: u32, c: u32 }
impl Argmin3Wide {
    fn run(&mut self) -> u16 {
        if self.b < self.a {
            if self.c < self.b { 2u16 } else { 1u16 }
        } else if self.c < self.a { 2u16 } else { 0u16 }
    }
}
