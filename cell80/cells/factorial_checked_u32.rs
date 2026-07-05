//! Factorial of n, checked: n! — escalates instead of silently wrapping once n! would exceed u32::MAX (n >= 13, since 13! overflows u32).
//! tags: math, factorial, combinatorics, checked, wide, u32, escalate, counting
//! entry: FactorialChecked::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if n! would exceed u32::MAX (n >= 13)
struct FactorialChecked { n: u32, result: u32 }
impl FactorialChecked {
    fn run(&mut self) -> u16 {
        let mut r = 1u32;
        let mut i = 2u32;
        while i <= self.n {
            let p = mul_checked_u32(r, i);
            r = p;
            i = i + 1u32;
        }
        self.result = r;
        1u16
    }
}
