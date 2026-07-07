//! Exact product of two u16 values as a wide u32 (never overflows: 65535*65535 fits u32). The math-campaign foundation cell — most checked arithmetic composes from this.
//! tags: math, multiply, wide, exact, checked, u32, product
//! entry: MulWide::run
struct MulWide { a: u16, b: u16, product: u32 }
impl MulWide {
    fn run(&mut self) -> u16 {
        self.product = self.a as u32 * self.b as u32;
        if (self.product >> 16u32) as u16 != 0u16 { 65535u16 } else { self.product as u16 }
    }
}
