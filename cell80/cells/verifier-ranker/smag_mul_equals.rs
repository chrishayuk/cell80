//! Verifies a claimed sign-magnitude product: recomputes mag_a*mag_b via the wrapping-multiply-and-divide-back overflow-detect idiom product_equals_u32 uses, forms the expected sign as same-positive/different-negative (zero-magnitude canonicalizing to nonnegative, per smag_mul's own rule), and returns 1 if both match the claimed (mag_c, neg_c) else 0 — the reverse-equation counterpart smag_mul itself never got, unlike its unsigned sibling mul_checked_u32 (-> product_equals_u32).
//! tags: verify, verifier, equation, multiply, product, signed, sign-magnitude, wide, u32, check, plan, reverse-equation
//! entry: SmagMulEquals::run
struct SmagMulEquals { mag_a: u32, neg_a: u16, mag_b: u32, neg_b: u16, mag_c: u32, neg_c: u16 }
impl SmagMulEquals {
    fn run(&mut self) -> u16 {
        let p = self.mag_a.wrapping_mul(self.mag_b);
        if self.mag_a != 0u32 && p / self.mag_a != self.mag_b {
            0u16
        } else {
            let expected_neg = if p == 0u32 { 0u16 } else if self.neg_a == self.neg_b { 0u16 } else { 1u16 };
            let mut claimed_neg = self.neg_c;
            if self.mag_c == 0u32 { claimed_neg = 0u16; }
            ((p == self.mag_c) && (expected_neg == claimed_neg)) as u16
        }
    }
}
