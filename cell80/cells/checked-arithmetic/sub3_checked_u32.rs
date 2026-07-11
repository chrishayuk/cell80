//! Checked three-way subtract at u32: a-b-c, escalating if either subtract step would go negative — the exact, wide sibling arity-3 sub, matching add3_checked_u32/mul3_checked_u32.
//! tags: math, subtract, three, sub3, triple, checked, wide, u32, negative, escalate, rate, net-rate
//! entry: Sub3Checked::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if b > a or c > (a-b)
struct Sub3Checked { a: u32, b: u32, c: u32, diff: u32 }
impl Sub3Checked {
    fn run(&mut self) -> u16 {
        let d1 = sub_checked_u32(self.a, self.b);
        let d2 = sub_checked_u32(d1, self.c);
        self.diff = d2;
        1u16
    }
}
