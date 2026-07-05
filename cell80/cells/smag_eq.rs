//! Verifies whether two signed values (magnitude, sign pairs, per smag_add) are equal, canonicalizing negative-zero to nonnegative first — the sign-magnitude counterpart of frac_eq / answer_eq_u32.
//! tags: verify, verifier, equal, equation, signed, sign-magnitude, check, plan
//! entry: SmagEq::run
struct SmagEq { mag_a: u32, neg_a: u16, mag_b: u32, neg_b: u16 }
impl SmagEq {
    fn run(&mut self) -> u16 {
        let mut sa = self.neg_a;
        if self.mag_a == 0u32 { sa = 0u16; }
        let mut sb = self.neg_b;
        if self.mag_b == 0u32 { sb = 0u16; }
        (self.mag_a == self.mag_b && sa == sb) as u16
    }
}
