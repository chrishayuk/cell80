//! Median of four wide u32 values: average of the two middle order statistics via the same sorting network median4 uses, combined with the overflow-safe (lo & hi) + ((lo ^ hi) >> 1) trick at u32 width — the wide sibling of median4, mirroring min4_u32's relationship to min4.
//! tags: median, middle, four, stat, midpoint, central, order-statistic, wide, u32, large
//! entry: Median4Wide::run
struct Median4Wide { a: u32, b: u32, c: u32, d: u32, result: u32 }
impl Median4Wide {
    fn run(&mut self) -> u16 {
        let lo1 = if self.a < self.b { self.a } else { self.b };
        let hi1 = if self.a < self.b { self.b } else { self.a };
        let lo2 = if self.c < self.d { self.c } else { self.d };
        let hi2 = if self.c < self.d { self.d } else { self.c };
        let mid_lo = if lo1 > lo2 { lo1 } else { lo2 };
        let mid_hi = if hi1 < hi2 { hi1 } else { hi2 };
        self.result = (mid_lo & mid_hi) + ((mid_lo ^ mid_hi) >> 1u32);
        1u16
    }
}
