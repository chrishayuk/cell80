//! Returns 1 if x is nonzero at wide u32 width, else 0 — the wide sibling of nonzero (which works over u16 and can't represent values beyond 65535, e.g. money totals in cents).
//! tags: predicate, nonzero, present, truthy, set, boolean, wide, u32, large
//! entry: NonzeroWide::run
struct NonzeroWide { x: u32 }
impl NonzeroWide {
    fn run(&mut self) -> u16 {
        (self.x != 0u32) as u16
    }
}
