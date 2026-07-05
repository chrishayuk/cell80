//! Verifies a claimed wide four-way sum: returns 1 if a + b + c + d == total, else 0, without escalating on overflow — the missing four-way sibling of sum3_equals_u32 (a real gap: every prior verifier-ranker sum shape topped out at three parts).
//! tags: verify, verifier, equation, sum, addition, four, parts, wide, u32, check, plan, reverse-equation
//! entry: PartsSumToTotal4Wide::run
struct PartsSumToTotal4Wide { a: u32, b: u32, c: u32, d: u32, total: u32 }
impl PartsSumToTotal4Wide {
    fn run(&mut self) -> u16 {
        let s1 = self.a.wrapping_add(self.b);
        if s1 < self.a {
            0u16
        } else {
            let s2 = s1.wrapping_add(self.c);
            if s2 < s1 {
                0u16
            } else {
                let s3 = s2.wrapping_add(self.d);
                if s3 < s2 { 0u16 } else { (s3 == self.total) as u16 }
            }
        }
    }
}
