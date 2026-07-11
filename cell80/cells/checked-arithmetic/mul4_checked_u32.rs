//! Checked four-way multiply at u32: a*b*c*d, escalating the moment any sequential multiply step overflows — the wide four-term sibling of mul3_checked_u32 (composes mul_checked_u32 three times), matching add4_checked_u32's arity.
//! tags: math, multiply, four, mul4, quad, product, checked, wide, u32, overflow, escalate, volume, rate
//! entry: Mul4Checked::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if a*b, (a*b)*c, or (a*b*c)*d would exceed u32::MAX
struct Mul4Checked { a: u32, b: u32, c: u32, d: u32, product: u32 }
impl Mul4Checked {
    fn run(&mut self) -> u16 {
        let p1 = mul_checked_u32(self.a, self.b);
        let p2 = mul_checked_u32(p1, self.c);
        let p3 = mul_checked_u32(p2, self.d);
        self.product = p3;
        1u16
    }
}
