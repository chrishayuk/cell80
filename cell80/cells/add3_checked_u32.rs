//! Checked three-way add at u32: a+b+c, escalating if either add step overflows — the exact, wide sibling of sum3 (which saturates at u16).
//! tags: math, add, three, add3, triple, sum, total, checked, wide, u32, overflow, escalate
//! entry: Add3Checked::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if a+b or (a+b)+c would exceed u32::MAX
struct Add3Checked { a: u32, b: u32, c: u32, sum: u32 }
impl Add3Checked {
    fn run(&mut self) -> u16 {
        let s1 = self.a.wrapping_add(self.b);
        if s1 < self.a { halt(0xFF05u16); }
        let s2 = s1.wrapping_add(self.c);
        if s2 < s1 { halt(0xFF05u16); }
        self.sum = s2;
        1u16
    }
}
