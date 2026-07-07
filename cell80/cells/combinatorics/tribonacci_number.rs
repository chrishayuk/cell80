//! The nth Tribonacci number (T(0)=0, T(1)=1, T(2)=1, T(n)=T(n-1)+T(n-2)+T(n-3)), checked: escalates instead of silently wrapping once T(n) would exceed u32::MAX. Distinct from fibonacci_checked_u32's two-term recurrence -- a genuinely different sequence, not reducible to lucas_u_v's two-term p/q family.
//! tags: number, tribonacci, sequence, recurrence, combinatorics, checked, wide, u32, escalate
//! entry: TribonacciChecked::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if T(n) would exceed u32::MAX
struct TribonacciChecked { n: u32, result: u32 }
impl TribonacciChecked {
    fn run(&mut self) -> u16 {
        let mut a = 0u32;
        let mut b = 1u32;
        let mut c = 1u32;
        if self.n == 0u32 {
            self.result = a;
        } else if self.n == 1u32 {
            self.result = b;
        } else if self.n == 2u32 {
            self.result = c;
        } else {
            let mut i = 2u32;
            while i < self.n {
                let next = add_checked_u32(add_checked_u32(a, b), c);
                a = b;
                b = c;
                c = next;
                i = i + 1u32;
            }
            self.result = c;
        }
        1u16
    }
}
