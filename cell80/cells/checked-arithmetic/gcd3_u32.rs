//! Greatest common divisor of three wide u32 values via two chained inline Euclidean loops, gcd(gcd(a,b),c) — the wide sibling of gcd_u32, and the arity-3 extension gcd_u32 lacks (mirroring gcd3, which is u16-only and can't represent divisors beyond 65535).
//! tags: number, gcd, divisor, common, factor, highest, three, gcd3, wide, u32, large, euclidean
//! entry: Gcd3Wide::run
struct Gcd3Wide { a: u32, b: u32, c: u32, result: u32 }
impl Gcd3Wide {
    fn run(&mut self) -> u16 {
        let mut x = self.a;
        let mut y = self.b;
        while y != 0u32 {
            let t = y;
            y = x % y;
            x = t;
        }
        let mut u = x;
        let mut v = self.c;
        while v != 0u32 {
            let t = v;
            v = u % v;
            u = t;
        }
        self.result = u;
        1u16
    }
}
