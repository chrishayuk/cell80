//! Least common multiple of three wide u32 values via two chained inline gcd-then-checked-multiply steps, lcm(lcm(a,b),c), 0 if any input is 0 — the wide sibling of lcm_u32 at arity 3, mirroring lcm3 (u16-only, can't represent multiples beyond 65535).
//! tags: number, lcm, multiple, common, divisor, three, lcm3, wide, u32, checked, overflow, escalate, large
//! entry: Lcm3Checked::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if either lcm step's multiply would overflow u32
struct Lcm3Checked { a: u32, b: u32, c: u32, result: u32 }
impl Lcm3Checked {
    fn run(&mut self) -> u16 {
        if self.a == 0u32 || self.b == 0u32 || self.c == 0u32 {
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
        let g1 = x;
        let q1 = self.a / g1;
        let l1 = mul_checked_u32(q1, self.b);

        let mut u = l1;
        let mut v = self.c;
        while v != 0u32 {
            let t = v;
            v = u % v;
            u = t;
        }
        let g2 = u;
        let q2 = l1 / g2;
        self.result = mul_checked_u32(q2, self.c);
        1u16
    }
}
