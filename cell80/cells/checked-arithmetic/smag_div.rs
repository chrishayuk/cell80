//! Divide two signed values exactly: magnitudes divide (escalating on a nonzero remainder), sign is same-positive/different-negative (per smag_add).
//! tags: math, signed, sign-magnitude, divide, quotient, wide, u32, checked, escalate, exact
//! entry: SmagDiv::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if mag_b == 0 or mag_a doesn't divide evenly by mag_b; escalates (halt 0xFF06, out_of_domain) if neg_a or neg_b is anything other than 0 or 1
struct SmagDiv { mag_a: u32, neg_a: u16, mag_b: u32, neg_b: u16, mag: u32, neg: u16 }
impl SmagDiv {
    fn run(&mut self) -> u16 {
        if self.neg_a > 1u16 || self.neg_b > 1u16 { halt(0xFF06u16); }
        if self.mag_b == 0u32 { halt(0xFF05u16); }
        if self.mag_a % self.mag_b != 0u32 { halt(0xFF05u16); }
        self.mag = self.mag_a / self.mag_b;
        let n = if self.mag == 0u32 { 0u16 } else if self.neg_a == self.neg_b { 0u16 } else { 1u16 };
        self.neg = n;
        1u16
    }
}
