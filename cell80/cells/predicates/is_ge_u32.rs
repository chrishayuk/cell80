//! Returns 1 if a >= b (at least) at wide u32 width, else 0 — the wide sibling of is_ge (which works over u16 and can't compare values beyond 65535, e.g. money totals in cents).
//! tags: predicate, compare, greater-equal, ge, order, wide, u32, large
//! entry: IsGeWide::run
struct IsGeWide { a: u32, b: u32 }
impl IsGeWide {
    fn run(&mut self) -> u16 {
        (self.a >= self.b) as u16
    }
}
