//! Smallest of four wide u32 values, written to a result field — extends min3_u32's pairwise-min chain one level deeper; the value-returning counterpart of argmin4_u32, which returns the winning index instead of the value.
//! tags: math, min, minimum, four, min4, quad, smallest, least, lesser, compare, select, wide, u32, large
//! entry: Min4Wide::run
struct Min4Wide { a: u32, b: u32, c: u32, d: u32, result: u32 }
impl Min4Wide {
    fn run(&mut self) -> u16 {
        let m1 = if self.a < self.b { self.a } else { self.b };
        let m2 = if m1 < self.c { m1 } else { self.c };
        let m3 = if m2 < self.d { m2 } else { self.d };
        self.result = m3;
        1u16
    }
}
