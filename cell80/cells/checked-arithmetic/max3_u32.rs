//! Largest of three wide u32 values — the exact wide sibling of max3 (which works over u16 and can't rank values beyond 65535, e.g. money totals in cents).
//! tags: math, max, maximum, largest, greatest, extremum, three, compare, select, wide, u32, large
//! entry: Max3Wide::run
struct Max3Wide { a: u32, b: u32, c: u32, result: u32 }
impl Max3Wide {
    fn run(&mut self) -> u16 {
        let m1 = if self.a > self.b { self.a } else { self.b };
        let m2 = if m1 > self.c { m1 } else { self.c };
        self.result = m2;
        1u16
    }
}
