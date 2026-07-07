//! Constraint check for a signed-magnitude quantity (magnitude, sign pair — neg 0=nonnegative, 1=negative, per smag_add): returns 1 if the value is nonnegative (neg == 0, or magnitude == 0 regardless of the sign flag), else 0.
//! tags: verify, verifier, nonneg, constraint, signed, sign-magnitude, wide, u32, check, plan
//! entry: SmagIsNonneg::run
struct SmagIsNonneg { mag: u32, neg: u16, ok: u16 }
impl SmagIsNonneg {
    fn run(&mut self) -> u16 {
        let r = if self.mag == 0u32 { 1u16 } else { (self.neg == 0u16) as u16 };
        self.ok = r;
        r
    }
}
