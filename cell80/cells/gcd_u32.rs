//! Greatest common divisor of two wide u32 values via an inline Euclidean loop — the wide sibling of gcd (which works over u16 and can't represent divisors beyond 65535).
//! tags: number, gcd, divisor, common, factor, highest, wide, u32, large
//! entry: GcdWide::run
struct GcdWide { a: u32, b: u32, result: u32 }
impl GcdWide {
    fn run(&mut self) -> u16 {
        let mut x = self.a;
        let mut y = self.b;
        while y != 0u32 {
            let t = y;
            y = x % y;
            x = t;
        }
        self.result = x;
        1u16
    }
}
