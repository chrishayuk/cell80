//! Sign-magnitude subtract: a - b for two signed quantities represented as (magnitude, sign) pairs (neg 0=nonnegative, 1=negative, per smag_add) — computed by flipping b's sign and adding, the same rule table as smag_add. Escalates on magnitude overflow.
//! tags: math, signed, sign-magnitude, subtract, wide, u32, checked, escalate
//! entry: SmagSub::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if the same-sign magnitudes would overflow u32; escalates (halt 0xFF06, out_of_domain) if neg_a or neg_b is anything other than 0 or 1
struct SmagSub { mag_a: u32, neg_a: u16, mag_b: u32, neg_b: u16, mag: u32, neg: u16 }
impl SmagSub {
    fn run(&mut self) -> u16 {
        if self.neg_a > 1u16 || self.neg_b > 1u16 { halt(0xFF06u16); }
        let nb = 1u16 - self.neg_b;
        if self.neg_a == nb {
            let s = add_checked_u32(self.mag_a, self.mag_b);
            self.mag = s;
            self.neg = self.neg_a;
        } else if self.mag_a >= self.mag_b {
            let d = self.mag_a - self.mag_b;
            self.mag = d;
            let n = if d == 0u32 { 0u16 } else { self.neg_a };
            self.neg = n;
        } else {
            self.mag = self.mag_b - self.mag_a;
            self.neg = nb;
        }
        1u16
    }
}
