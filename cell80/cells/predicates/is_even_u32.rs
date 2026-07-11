//! Returns 1 if a wide u32 value is even, else 0 — the wide sibling of is_even (which works over u16 and can't hold values beyond 65535, e.g. money totals in cents).
//! tags: predicate, even, parity, divisible-by-two, boolean, wide, u32, large
//! entry: IsEvenWide::run
struct IsEvenWide { x: u32 }
impl IsEvenWide {
    fn run(&mut self) -> u16 {
        (self.x % 2u32 == 0u32) as u16
    }
}
