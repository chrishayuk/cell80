//! Checked three-way multiply at u32: a*b*c, escalating if either multiply step overflows (e.g. a box volume: length*width*height).
//! tags: math, multiply, three, checked, wide, u32, overflow, volume, escalate
//! entry: Mul3Checked::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if a*b or (a*b)*c would exceed u32::MAX
struct Mul3Checked { a: u32, b: u32, c: u32, product: u32 }
impl Mul3Checked {
    fn run(&mut self) -> u16 {
        let p1 = mul_checked_u32(self.a, self.b);
        let p2 = mul_checked_u32(p1, self.c);
        self.product = p2;
        1u16
    }
}
