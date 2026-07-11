//! Returns 1 if lo <= x < hi (half-open interval: closed at lo, open at hi) at wide u32 width, else 0 — the wide sibling of in_range_closed_open (which works over u16).
//! tags: validation, validate, range, bounds, check, half-open, interval, wide, u32, large
//! entry: InRangeClosedOpenWide::run
struct InRangeClosedOpenWide { x: u32, lo: u32, hi: u32 }
impl InRangeClosedOpenWide {
    fn run(&mut self) -> u16 {
        (self.lo <= self.x && self.x < self.hi) as u16
    }
}
