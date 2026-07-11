//! Returns 1 if lo < x < hi (strictly inside, exclusive bounds) at wide u32 width, else 0 — the wide sibling of between_exclusive (which works over u16 and can't compare values beyond 65535, e.g. money totals in cents).
//! tags: bounds, between, exclusive, strictly, interval, open, wide, u32, large
//! entry: BetweenExclusiveWide::run
struct BetweenExclusiveWide { x: u32, lo: u32, hi: u32 }
impl BetweenExclusiveWide {
    fn run(&mut self) -> u16 {
        ((self.lo < self.x) && (self.x < self.hi)) as u16
    }
}
