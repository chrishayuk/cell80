//! Sum of four values (saturating at 65535) — the four-operand sibling of sum3.
//! tags: sum, add, total, four, aggregate, accumulate
//! entry: Sum4::run
struct Sum4 { a: u16, b: u16, c: u16, d: u16, total: u16 }
impl Sum4 {
    fn run(&mut self) -> u16 {
        let ab = self.a.wrapping_add(self.b);
        let t1 = if ab < self.a { 65535u16 } else { ab };
        let abc = t1.wrapping_add(self.c);
        let t2 = if abc < t1 { 65535u16 } else { abc };
        let s = t2.wrapping_add(self.d);
        let t3 = if s < t2 { 65535u16 } else { s };
        self.total = t3;
        1u16
    }
}
