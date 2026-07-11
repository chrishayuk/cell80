//! The double factorial n!! = n*(n-2)*(n-4)*...*(2 or 1) (0!! = 1!! = 1 by convention), checked -- escalates instead of silently wrapping once n!! would exceed u32::MAX, the same checked-recurrence shape factorial_checked_u32 uses but skipping every other term (a genuinely distinct sequence, not reducible to n! by any simple formula for general n).
//! tags: math, factorial, double factorial, combinatorics, checked, wide, u32, escalate, counting
//! entry: DoubleFactorial::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if n!! would exceed u32::MAX (n >= 21 for odd n, n >= 22 for even n)
struct DoubleFactorial { n: u32, result: u32 }
impl DoubleFactorial {
    fn run(&mut self) -> u16 {
        let mut r = 1u32;
        let mut i = self.n;
        while i >= 2u32 {
            let p = mul_checked_u32(r, i);
            r = p;
            i = i - 2u32;
        }
        self.result = r;
        1u16
    }
}
