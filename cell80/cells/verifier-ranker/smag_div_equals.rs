//! Verifies a claimed sign-magnitude quotient: 0 if mag_b == 0 or mag_a doesn't divide mag_b evenly, else 1 iff mag_a / mag_b equals the claimed magnitude and the sign matches same-positive/different-negative (zero-magnitude canonicalized to nonnegative) -- the verifier counterpart of smag_div (which escalates on a nonzero remainder; this one always returns a verdict), completing the smag_add/sub/mul/div reverse-equation family.
//! tags: verify, verifier, equation, quotient, divide, signed, sign-magnitude, wide, u32, check, plan, reverse-equation
//! entry: SmagDivEquals::run
struct SmagDivEquals { mag_a: u32, neg_a: u16, mag_b: u32, neg_b: u16, mag_c: u32, neg_c: u16 }
impl SmagDivEquals {
    fn run(&mut self) -> u16 {
        if self.mag_b == 0u32 {
            0u16
        } else if self.mag_a % self.mag_b != 0u32 {
            0u16
        } else {
            let q = self.mag_a / self.mag_b;
            let expected_neg = if q == 0u32 { 0u16 } else if self.neg_a == self.neg_b { 0u16 } else { 1u16 };
            let mut claimed_neg = self.neg_c;
            if self.mag_c == 0u32 { claimed_neg = 0u16; }
            (q == self.mag_c && expected_neg == claimed_neg) as u16
        }
    }
}
