//! Checked three-way add at u32: a+b+c, escalating if either add step overflows — the exact, wide sibling of sum3 (which saturates at u16).
//! tags: math, add, three, add3, triple, sum, total, checked, wide, u32, overflow, escalate, rate, combined-rate
//! entry: Add3Checked::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if a+b or (a+b)+c would exceed u32::MAX
struct Add3Checked { a: u32, b: u32, c: u32, sum: u32 }
impl Add3Checked {
    fn run(&mut self) -> u16 {
        let s1 = add_checked_u32(self.a, self.b);
        let s2 = add_checked_u32(s1, self.c);
        self.sum = s2;
        1u16
    }
}
