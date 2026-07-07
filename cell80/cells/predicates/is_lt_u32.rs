//! Returns 1 if a < b (strictly less than) at wide u32 width, else 0 — the wide sibling of is_lt (which works over u16 and can't compare values beyond 65535, e.g. money totals in cents).
//! tags: predicate, compare, less, lt, order, wide, u32, large
//! entry: IsLtWide::run
struct IsLtWide { a: u32, b: u32 }
impl IsLtWide {
    fn run(&mut self) -> u16 {
        (self.a < self.b) as u16
    }
}
