//! Returns 1 if a <= b (at most) at wide u32 width, else 0 — the wide sibling of is_le (which works over u16 and can't compare values beyond 65535, e.g. money totals in cents).
//! tags: predicate, compare, less-equal, le, order, wide, u32, large
//! entry: IsLeWide::run
struct IsLeWide { a: u32, b: u32 }
impl IsLeWide {
    fn run(&mut self) -> u16 {
        (self.a <= self.b) as u16
    }
}
