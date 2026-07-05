//! Returns 1 if a divides b evenly at wide u32 width (b % a == 0, a != 0), else 0 — the wide sibling of divides (which works over u16).
//! tags: number, divides, divisible, factor, predicate, multiple, wide, u32, large
//! entry: DividesWide::run
struct DividesWide { a: u32, b: u32, ok: u16 }
impl DividesWide {
    fn run(&mut self) -> u16 {
        let r = if self.a != 0u32 { (self.b % self.a == 0u32) as u16 } else { 0u16 };
        self.ok = r;
        r
    }
}
