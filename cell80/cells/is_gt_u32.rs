//! Returns 1 if a > b (strictly greater than) at wide u32 width, else 0 — the wide sibling of is_gt (which works over u16 and can't compare values beyond 65535, e.g. money totals in cents).
//! tags: predicate, compare, greater, gt, order, wide, u32, large
//! entry: IsGtWide::run
struct IsGtWide { a: u32, b: u32 }
impl IsGtWide {
    fn run(&mut self) -> u16 {
        (self.a > self.b) as u16
    }
}
