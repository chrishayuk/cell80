//! Verifies a claimed wide three-way sum: returns 1 if a + b + c == total, else 0, without escalating on overflow — the reverse-equation counterpart of add3_checked_u32.
//! tags: verify, verifier, equation, sum, addition, three, wide, u32, check, plan, reverse-equation
//! entry: Sum3EqualsWide::run
struct Sum3EqualsWide { a: u32, b: u32, c: u32, total: u32 }
impl Sum3EqualsWide {
    fn run(&mut self) -> u16 {
        let s1 = self.a.wrapping_add(self.b);
        if s1 < self.a {
            0u16
        } else {
            let s2 = s1.wrapping_add(self.c);
            if s2 < s1 { 0u16 } else { (s2 == self.total) as u16 }
        }
    }
}
