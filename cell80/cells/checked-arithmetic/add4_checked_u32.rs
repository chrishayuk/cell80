//! Checked four-way add at u32: a+b+c+d, escalating the moment any sequential add step overflows — the wide four-term sibling of add3_checked_u32 (composes add_checked_u32 three times), and the compute-side counterpart of parts_sum_to_total4_u32's verify-side check.
//! tags: math, add, four, add4, quad, sum, total, checked, wide, u32, overflow, escalate, rate, combined-rate
//! entry: Add4Checked::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if a+b, (a+b)+c, or (a+b+c)+d would exceed u32::MAX
struct Add4Checked { a: u32, b: u32, c: u32, d: u32, sum: u32 }
impl Add4Checked {
    fn run(&mut self) -> u16 {
        let s1 = add_checked_u32(self.a, self.b);
        let s2 = add_checked_u32(s1, self.c);
        let s3 = add_checked_u32(s2, self.d);
        self.sum = s3;
        1u16
    }
}
