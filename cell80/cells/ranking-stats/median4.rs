//! Median of four values: average of the two middle order statistics via a 4-comparison sorting network, then averaged overflow-safely with midrange3's (mid_lo & mid_hi) + ((mid_lo ^ mid_hi) >> 1) trick — distinct from mean4 (order-statistic based, not sum-based) and the four-value sibling median3 never had.
//! tags: median, middle, four, stat, midpoint, central, order-statistic
//! entry: Median4::run
struct Median4 { a: u16, b: u16, c: u16, d: u16 }
impl Median4 {
    fn run(&mut self) -> u16 {
        let lo1 = imin(self.a, self.b);
        let hi1 = imax(self.a, self.b);
        let lo2 = imin(self.c, self.d);
        let hi2 = imax(self.c, self.d);
        let mid_lo = imax(lo1, lo2);
        let mid_hi = imin(hi1, hi2);
        (mid_lo & mid_hi) + ((mid_lo ^ mid_hi) >> 1u16)
    }
}
