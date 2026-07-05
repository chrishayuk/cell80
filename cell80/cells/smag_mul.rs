//! Multiply two signed values: magnitudes multiply (checked for overflow), sign is same-positive/different-negative (per smag_add).
//! tags: math, signed, sign-magnitude, multiply, product, times, wide, u32, checked, escalate
//! entry: SmagMul::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if the magnitudes overflow u32; escalates (halt 0xFF06, out_of_domain) if neg_a or neg_b is anything other than 0 or 1
struct SmagMul { mag_a: u32, neg_a: u16, mag_b: u32, neg_b: u16, mag: u32, neg: u16 }
impl SmagMul {
    fn run(&mut self) -> u16 {
        if self.neg_a > 1u16 || self.neg_b > 1u16 { halt(0xFF06u16); }
        let p = mul_checked_u32(self.mag_a, self.mag_b);
        self.mag = p;
        let n = if p == 0u32 { 0u16 } else if self.neg_a == self.neg_b { 0u16 } else { 1u16 };
        self.neg = n;
        1u16
    }
}
