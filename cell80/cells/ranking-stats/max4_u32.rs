//! Largest of four wide u32 values, written to a result field — the four-operand sibling of max3_u32, mirroring max4's relationship to max3 at u16 width.
//! tags: math, max, maximum, largest, greatest, extremum, four, compare, select, wide, u32, large
//! entry: Max4Wide::run
struct Max4Wide { a: u32, b: u32, c: u32, d: u32, result: u32 }
impl Max4Wide {
    fn run(&mut self) -> u16 {
        let m1 = if self.a > self.b { self.a } else { self.b };
        let m2 = if m1 > self.c { m1 } else { self.c };
        let m3 = if m2 > self.d { m2 } else { self.d };
        self.result = m3;
        1u16
    }
}
