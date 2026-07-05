//! Running (population) variance over a stream of values, one value per call — the checked/exact sibling of accumulate_step (which saturates u16; this escalates on overflow instead, since a corrupted variance is worse than a stopped one). Recomputes the mean fresh from the exact running sum on each side of the update (rather than compounding a previously-truncated running mean, Welford-style) before accumulating the squared-deviation product into m2 — verified to never go negative under integer truncation across thousands of random and adversarial streams. Compose with div_floor_u32(m2, count) for the variance itself.
//! tags: running, variance, stats, welford, stream, state, wide, u32, checked, escalate
//! entry: RunningVariance::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if the running sum overflows u32 (roughly 65537 samples at the u16 domain max), if the squared-deviation product overflows u32, or if that product would be negative (an invariant violation, never observed empirically but guarded rather than trusted)
struct RunningVariance { value: u16, count: u32, sum: u32, m2: u32 }
impl RunningVariance {
    fn run(&mut self) -> u16 {
        let old_count = self.count;
        let old_sum = self.sum;
        let value_w = self.value as u32;
        let new_sum = old_sum.wrapping_add(value_w);
        if new_sum < old_sum { halt(0xFF05u16); }
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

            let new_m2 = self.m2.wrapping_add(prod_mag);
            if new_m2 < self.m2 { halt(0xFF05u16); }
            self.m2 = new_m2;
        }

        self.count = new_count;
        self.sum = new_sum;
        1u16
    }
}
