//! Sign-magnitude add: combine two signed quantities represented as (magnitude, sign) pairs — neg_a/neg_b are 0 (nonnegative) or 1 (negative), since the dialect has no i32 and this is how the math-campaign renderer tracks signed differences at u32 width (docs/math-campaign-spec.md). Escalates on magnitude overflow.
//! tags: math, signed, sign-magnitude, add, wide, u32, checked, escalate
//! entry: SmagAdd::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if the same-sign magnitudes would overflow u32; escalates (halt 0xFF06, out_of_domain) if neg_a or neg_b is anything other than 0 or 1
struct SmagAdd { mag_a: u32, neg_a: u16, mag_b: u32, neg_b: u16, mag: u32, neg: u16 }
impl SmagAdd {
    fn run(&mut self) -> u16 {
        if self.neg_a > 1u16 || self.neg_b > 1u16 { halt(0xFF06u16); }
        if self.neg_a == self.neg_b {
            let s = self.mag_a.wrapping_add(self.mag_b);
            if s < self.mag_a { halt(0xFF05u16); }
            self.mag = s;
            self.neg = self.neg_a;
        } else if self.mag_a >= self.mag_b {
            let d = self.mag_a - self.mag_b;
            self.mag = d;
            let n = if d == 0u32 { 0u16 } else { self.neg_a };
            self.neg = n;
        } else {
            self.mag = self.mag_b - self.mag_a;
            self.neg = self.neg_b;
        }
        1u16
    }
}
