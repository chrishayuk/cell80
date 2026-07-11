//! Apply a signed delta (tracked as a mag/neg pair, not i16) to a u32 value, clamped to [0, cap] at u32 width -- the wide sibling of apply_delta_clamped for a resource/health/balance pool too large for its u16 ceiling of 65535, mirroring its same-sign-add / opposite-sign-subtract clamp logic exactly.
//! tags: delta, signed, wide, u32, clamp, risk, adjust, health, resource, balance, bounds
//! entry: ApplyDeltaClampedWide::run
struct ApplyDeltaClampedWide { value: u32, delta_mag: u32, delta_neg: u16, cap: u32, result: u32 }
impl ApplyDeltaClampedWide {
    fn run(&mut self) -> u16 {
        if self.delta_neg == 0u16 {
            let sum = self.value.wrapping_add(self.delta_mag);
            let r = if sum < self.value || sum > self.cap { self.cap } else { sum };
            self.result = r;
        } else {
            let r = if self.delta_mag > self.value { 0u32 } else { self.value - self.delta_mag };
            self.result = r;
        }
        1u16
    }
}
