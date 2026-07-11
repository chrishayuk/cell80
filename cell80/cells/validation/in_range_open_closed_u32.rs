//! Returns 1 if lo < x <= hi at wide u32 width, else 0 — the wide sibling of in_range_open_closed (which works over u16).
//! tags: validation, validate, range, bounds, interval, check, wide, u32, large, half-open
//! entry: InRangeOpenClosedWide::run
struct InRangeOpenClosedWide { x: u32, lo: u32, hi: u32, ok: u16 }
impl InRangeOpenClosedWide {
    fn run(&mut self) -> u16 {
        self.ok = (self.lo < self.x && self.x <= self.hi) as u16;
        self.ok
    }
}
