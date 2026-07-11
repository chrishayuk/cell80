//! Verifies a claimed wide GCD: recomputes gcd(a, b) via the same inline Euclidean loop gcd_u32 uses and returns 1 if it equals the claimed g, else 0 — the reverse-equation counterpart of gcd_u32 (never halts, always a verdict).
//! tags: verify, verifier, equation, gcd, divisor, common, factor, euclidean, wide, u32, check, plan, reverse-equation
//! entry: GcdEqualsWide::run
struct GcdEqualsWide { a: u32, b: u32, g: u32 }
impl GcdEqualsWide {
    fn run(&mut self) -> u16 {
        let mut x = self.a;
        let mut y = self.b;
        while y != 0u32 {
            let t = y;
            y = x % y;
            x = t;
        }
        (x == self.g) as u16
    }
}
