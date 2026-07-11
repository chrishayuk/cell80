//! Returns 1 if a != b (not equal) at wide u32 width, else 0 — the wide sibling of neq (which works over u16 and can't compare values beyond 65535, e.g. money totals in cents).
//! tags: predicate, compare, not-equal, different, differs, wide, u32, large
//! entry: NeqWide::run
struct NeqWide { a: u32, b: u32 }
impl NeqWide {
    fn run(&mut self) -> u16 {
        (self.a != self.b) as u16
    }
}
