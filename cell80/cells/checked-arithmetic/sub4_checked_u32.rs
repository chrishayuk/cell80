//! Checked four-way subtract at u32: a-b-c-d, escalating the moment any sequential subtract step goes negative — the wide four-term sibling of sub3_checked_u32 (composes sub_checked_u32 three times), filling the arity-4 gap left open in the sub triad while add_checked_u32 already has both.
//! tags: math, subtract, four, sub4, quad, diff, checked, wide, u32, negative, escalate, rate, net-rate
//! entry: Sub4Checked::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if b > a, c > (a-b), or d > (a-b-c)
struct Sub4Checked { a: u32, b: u32, c: u32, d: u32, diff: u32 }
impl Sub4Checked {
    fn run(&mut self) -> u16 {
        let d1 = sub_checked_u32(self.a, self.b);
        let d2 = sub_checked_u32(d1, self.c);
        let d3 = sub_checked_u32(d2, self.d);
        self.diff = d3;
        1u16
    }
}
