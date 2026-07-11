//! The sqrt-of-variance sibling of running_variance_step: same running (count, sum, m2) update per value, then variance = m2/count (guarded) and stddev = floor(sqrt(variance)) via the same branch-free bitwise integer square root q_sqrt uses, inlined here for a raw (non-Q8.8-scaled) u32 rather than called across a pack boundary.
//! tags: running, stddev, standard-deviation, variance, stats, welford, stream, state, wide, u32, checked, escalate, sqrt
//! entry: RunningStddev::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if the running sum overflows u32, if the squared-deviation product overflows u32, or if that product would be negative (an invariant violation, guarded rather than trusted) — inherited unchanged from running_variance_step's update
struct RunningStddev { value: u16, count: u32, sum: u32, m2: u32, stddev: u16 }
impl RunningStddev {
    fn run(&mut self) -> u16 {
        let old_count = self.count;
        let old_sum = self.sum;
        let value_w = self.value as u32;
        let new_sum = add_checked_u32(old_sum, value_w);
        let new_count = old_count + 1u32;

        if old_count != 0u32 {
            let old_mean = old_sum / old_count;
            let new_mean = new_sum / new_count;
            let mag_do = if value_w >= old_mean { value_w - old_mean } else { old_mean - value_w };
            let neg_do = if value_w >= old_mean { 0u16 } else { 1u16 };
            let mag_dn = if value_w >= new_mean { value_w - new_mean } else { new_mean - value_w };
            let neg_dn = if value_w >= new_mean { 0u16 } else { 1u16 };

            let prod_mag = mag_do.wrapping_mul(mag_dn);
            if mag_do != 0u32 && prod_mag / mag_do != mag_dn { halt(0xFF05u16); }
            if neg_do != neg_dn && prod_mag != 0u32 { halt(0xFF05u16); }

            let new_m2 = add_checked_u32(self.m2, prod_mag);
            self.m2 = new_m2;
        }

        self.count = new_count;
        self.sum = new_sum;

        let variance = if self.count != 0u32 { self.m2 / self.count } else { 0u32 };

        // Branch-free bitwise integer square root of `variance` directly (no Q8.8 scale-up:
        // this is a raw u32 sqrt, not q_sqrt's sqrt(x/256)*256), inlined so the u32 magnitude
        // never has to cross a call boundary (u32 is state-cell-local only).
        let mut val = variance;
        let mut res = 0u32;
        let mut bit = 1u32 << 30u32;
        while bit > val {
            bit = bit >> 2u32;
        }
        while bit != 0u32 {
            if val >= res + bit {
                val = val - (res + bit);
                res = (res >> 1u32) + bit;
            } else {
                res = res >> 1u32;
            }
            bit = bit >> 2u32;
        }
        let stddev = res as u16;
        self.stddev = stddev;
        stddev
    }
}
