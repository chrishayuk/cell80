//! Spread of four wide u32 values: max4_u32 − min4_u32, written to a result field — the four-operand sibling of range3_u32, mirroring range4's relationship to range3 at u16 width.
//! tags: range, spread, span, stat, four, extent, wide, u32, large
//! entry: Range4Wide::run
struct Range4Wide { a: u32, b: u32, c: u32, d: u32, result: u32 }
impl Range4Wide {
    fn run(&mut self) -> u16 {
        let mx1 = if self.a > self.b { self.a } else { self.b };
        let mx2 = if mx1 > self.c { mx1 } else { self.c };
        let mx3 = if mx2 > self.d { mx2 } else { self.d };
        let mn1 = if self.a < self.b { self.a } else { self.b };
        let mn2 = if mn1 < self.c { mn1 } else { self.c };
        let mn3 = if mn2 < self.d { mn2 } else { self.d };
        self.result = mx3 - mn3;
        1u16
    }
}
