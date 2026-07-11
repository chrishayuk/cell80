//! Spread of three wide u32 values: max3_u32 − min3_u32, written to a result field — never underflows since max >= min by construction; the wide sibling of range3, which is u16-only and can't span totals past 65535 (e.g. money in cents).
//! tags: math, range, spread, span, stat, extent, three, triple, wide, u32, large, subtract, difference
//! entry: Range3Wide::run
struct Range3Wide { a: u32, b: u32, c: u32, result: u32 }
impl Range3Wide {
    fn run(&mut self) -> u16 {
        let hi1 = if self.a > self.b { self.a } else { self.b };
        let hi = if hi1 > self.c { hi1 } else { self.c };
        let lo1 = if self.a < self.b { self.a } else { self.b };
        let lo = if lo1 < self.c { lo1 } else { self.c };
        self.result = hi - lo;
        1u16
    }
}
