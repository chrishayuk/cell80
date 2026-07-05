//! Least common multiple of two wide u32 values via an inline GCD (0 if either is 0), escalating on overflow — unlike lcm (u16, silently wraps on overflow), this is the exact, checked wide sibling.
//! tags: number, lcm, multiple, common, divisor, wide, u32, checked, overflow, escalate, large
//! entry: LcmChecked::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if a/gcd*b would exceed u32::MAX
struct LcmChecked { a: u32, b: u32, result: u32 }
impl LcmChecked {
    fn run(&mut self) -> u16 {
        if self.a == 0u32 || self.b == 0u32 {
            self.result = 0u32;
            return 1u16;
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
        let p = q.wrapping_mul(self.b);
        if q != 0u32 && p / q != self.b { halt(0xFF05u16); }
        self.result = p;
        1u16
    }
}
