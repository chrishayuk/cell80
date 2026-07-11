//! Returns 1 if x is outside the open interval (lo, hi): x <= lo || x >= hi, at wide u32 width -- the wide sibling of outside_range (which works over u16 and can't compare values beyond 65535), keeping the predicate/complement pair symmetric with between_exclusive_u32.
//! tags: bounds, outside, between, exclusive, interval, open, complement, wide, u32, large
//! entry: OutsideRangeWide::run
struct OutsideRangeWide { x: u32, lo: u32, hi: u32 }
impl OutsideRangeWide {
    fn run(&mut self) -> u16 {
        ((self.x <= self.lo) || (self.x >= self.hi)) as u16
    }
}
