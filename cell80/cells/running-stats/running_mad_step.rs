//! Running mean absolute deviation over a stream of values, one value per call — a textbook-distinct third dispersion measure alongside running_variance_step/running_stddev_step's squared-deviation ones: accumulates sum_abs_dev += |x_i - running_mean_at_time_i| (the mean after folding in x_i) each call, using plain absolute value rather than the squared-product machinery variance needs, so no old-mean/new-mean product or negative-guard is required. Compose with div_floor_u32(sum_abs_dev, count) for the MAD itself.
//! tags: running, mad, mean-absolute-deviation, dispersion, stats, stream, state, wide, u32, checked, escalate
//! entry: RunningMad::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if the running sum overflows u32, or if the running sum_abs_dev overflows u32
struct RunningMad { value: u16, count: u32, sum: u32, sum_abs_dev: u32 }
impl RunningMad {
    fn run(&mut self) -> u16 {
        let value_w = self.value as u32;
        let new_sum = add_checked_u32(self.sum, value_w);
        let new_count = self.count + 1u32;
        let mean = new_sum / new_count;
        let abs_dev = if value_w >= mean { value_w - mean } else { mean - value_w };
        let new_sum_abs_dev = add_checked_u32(self.sum_abs_dev, abs_dev);

        self.sum = new_sum;
        self.count = new_count;
        self.sum_abs_dev = new_sum_abs_dev;
        1u16
    }
}
