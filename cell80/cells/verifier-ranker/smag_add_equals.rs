//! Verifies a claimed sign-magnitude sum: recomputes smag_add's same-sign-add / opposite-sign-subtract rule on (mag_a,neg_a)+(mag_b,neg_b) and checks it against the claimed (mag_c,neg_c), canonicalizing zero-magnitude to nonnegative on both sides — the sign-magnitude reverse-equation counterpart of sum_equals_u32, never escalating (a wrapping add plus overflow-detect stands in for add_checked_u32's halt, since a real overflow just means the claim doesn't hold).
//! tags: verify, verifier, equal, equation, signed, sign-magnitude, add, wide, u32, check, plan, reverse-equation
//! entry: SmagAddEquals::run
struct SmagAddEquals { mag_a: u32, neg_a: u16, mag_b: u32, neg_b: u16, mag_c: u32, neg_c: u16 }
impl SmagAddEquals {
    fn run(&mut self) -> u16 {
        let mut claimed_neg = self.neg_c;
        if self.mag_c == 0u32 { claimed_neg = 0u16; }

        let result = if self.neg_a == self.neg_b {
            let s = self.mag_a.wrapping_add(self.mag_b);
            if s < self.mag_a {
                0u16
            } else {
                let mut n = self.neg_a;
                if s == 0u32 { n = 0u16; }
                ((s == self.mag_c) && (n == claimed_neg)) as u16
            }
        } else if self.mag_a >= self.mag_b {
            let d = self.mag_a - self.mag_b;
            let mut n = self.neg_a;
            if d == 0u32 { n = 0u16; }
            ((d == self.mag_c) && (n == claimed_neg)) as u16
        } else {
            let d = self.mag_b - self.mag_a;
            let mut n = self.neg_b;
            if d == 0u32 { n = 0u16; }
            ((d == self.mag_c) && (n == claimed_neg)) as u16
        };
        result
    }
}
