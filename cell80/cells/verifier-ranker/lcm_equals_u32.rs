//! Verifies a claimed wide LCM: 0 if a==0 or b==0 unless the claimed l is also 0 (matching lcm_u32's zero convention); otherwise recomputes gcd(a, b) inline and forms (a/g) * b via wrapping_mul with the same overflow-detection idiom product_equals_u32 uses, returning 1 if it equals l else 0 — the reverse-equation counterpart of lcm_u32.
//! tags: verify, verifier, equation, lcm, multiple, common, euclidean, wide, u32, check, plan, reverse-equation
//! entry: LcmEqualsWide::run
struct LcmEqualsWide { a: u32, b: u32, l: u32 }
impl LcmEqualsWide {
    fn run(&mut self) -> u16 {
        if self.a == 0u32 || self.b == 0u32 {
            return (self.l == 0u32) as u16;
        }
        let mut x = self.a;
        let mut y = self.b;
        while y != 0u32 {
            let t = y;
            y = x % y;
            x = t;
        }
        let g = x;
        let q = self.a / g;
        let product = q.wrapping_mul(self.b);
        if q != 0u32 && product / q != self.b {
            0u16
        } else {
            (product == self.l) as u16
        }
    }
}
