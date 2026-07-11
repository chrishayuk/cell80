//! Apply a signed delta to a signed value and clamp the sum to an explicit signed range [lo, hi] (lo may be negative), the sum tracked as a magnitude/sign pair throughout so an intermediate that overflows i16's own range never needs to be materialized before clamping -- the signed sibling of apply_delta_clamped/apply_delta_clamped_u32, neither of which can represent a range with a floor below zero.
//! tags: delta, signed, i16, clamp, bounds, range, adjust, wide, sign-magnitude, overflow
//! entry: ApplyDeltaClampedI16::run
fn i16_mag(v: i16) -> u32 {
    if v < 0i16 { (0u16.wrapping_sub(v as u16)) as u32 } else { v as u16 as u32 }
}
fn i16_neg(v: i16) -> u16 {
    if v < 0i16 { 1u16 } else { 0u16 }
}
struct ApplyDeltaClampedI16 { value: i16, delta: i16, lo: i16, hi: i16, result: i16 }
impl ApplyDeltaClampedI16 {
    fn run(&mut self) -> u16 {
        let value_mag = i16_mag(self.value);
        let value_neg = i16_neg(self.value);
        let delta_mag = i16_mag(self.delta);
        let delta_neg = i16_neg(self.delta);

        // sum = value + delta (the smag_add shape): same-sign is an add, opposite-sign
        // is a subtract keyed off the larger magnitude. value/delta are each i16-bounded
        // (max magnitude 32768) so the same-sign sum tops out at 65536, well inside u32 --
        // no add_checked_u32 needed, but the point is this sum can already exceed i16's
        // own +/-32768 range, which is exactly why it stays in mag/neg form.
        let mut sum_mag = 0u32;
        let mut sum_neg = 0u16;
        if value_neg == delta_neg {
            sum_mag = value_mag + delta_mag;
            sum_neg = value_neg;
        } else if value_mag >= delta_mag {
            sum_mag = value_mag - delta_mag;
            sum_neg = if sum_mag == 0u32 { 0u16 } else { value_neg };
        } else {
            sum_mag = delta_mag - value_mag;
            sum_neg = delta_neg;
        }

        let lo_mag = i16_mag(self.lo);
        let lo_neg = i16_neg(self.lo);
        let hi_mag = i16_mag(self.hi);
        let hi_neg = i16_neg(self.hi);

        // Clamp: compare the (possibly out-of-i16-range) sum against lo/hi entirely in
        // sign-magnitude space (inline, not a shared helper -- a 4-input signed compare
        // would need 4 call params, past the 3-register calling convention), only
        // narrowing back to i16 once a final answer is picked.
        let sum_lt_lo = if sum_neg != lo_neg {
            sum_neg == 1u16
        } else if sum_neg == 0u16 {
            sum_mag < lo_mag
        } else {
            sum_mag > lo_mag
        };
        let hi_lt_sum = if hi_neg != sum_neg {
            hi_neg == 1u16
        } else if hi_neg == 0u16 {
            hi_mag < sum_mag
        } else {
            hi_mag > sum_mag
        };

        let mut result_mag = sum_mag;
        let mut result_neg = sum_neg;
        if sum_lt_lo {
            result_mag = lo_mag;
            result_neg = lo_neg;
        } else if hi_lt_sum {
            result_mag = hi_mag;
            result_neg = hi_neg;
        }

        let r = if result_neg == 1u16 {
            (0u16.wrapping_sub(result_mag as u16)) as i16
        } else {
            result_mag as u16 as i16
        };
        self.result = r;
        1u16
    }
}
