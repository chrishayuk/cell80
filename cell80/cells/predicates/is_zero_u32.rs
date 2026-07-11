//! Returns 1 if a wide u32 value is zero, else 0 — the wide sibling of is_zero (which works over u16 and can't hold values beyond 65535, e.g. money totals in cents).
//! tags: predicate, zero, empty, is-zero, none, boolean, wide, u32, large
//! entry: IsZeroWide::run
struct IsZeroWide { x: u32 }
impl IsZeroWide {
    fn run(&mut self) -> u16 {
        (self.x == 0u32) as u16
    }
}
