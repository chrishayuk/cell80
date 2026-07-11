//! Returns 1 if a wide u32 value is odd, else 0 — the wide sibling of is_odd (which works over u16 and can't hold values beyond 65535, e.g. money totals in cents).
//! tags: predicate, odd, parity, boolean, wide, u32, large
//! entry: IsOddWide::run
struct IsOddWide { x: u32 }
impl IsOddWide {
    fn run(&mut self) -> u16 {
        (self.x % 2u32 != 0u32) as u16
    }
}
