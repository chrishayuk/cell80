//! Returns 1 if two wide u32 values are coprime (their gcd, via the same inline Euclidean loop gcd_u32 runs, equals 1), else 0 — the wide sibling of is_coprime (which works over u16).
//! tags: number, coprime, gcd, relatively-prime, predicate, divisor, euclidean, wide, u32, large
//! entry: IsCoprimeWide::run
struct IsCoprimeWide { a: u32, b: u32, ok: u16 }
impl IsCoprimeWide {
    fn run(&mut self) -> u16 {
        let mut x = self.a;
        let mut y = self.b;
        while y != 0u32 {
            let t = y;
            y = x % y;
            x = t;
        }
        let r = (x == 1u32) as u16;
        self.ok = r;
        r
    }
}
