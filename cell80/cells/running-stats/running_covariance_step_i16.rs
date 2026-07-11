//! Signed sibling of running_covariance_step: running (stream) accumulation of sum_x, sum_y, sum_xy for a stream of signed (i16, i16) pairs, each sum tracked as a (magnitude, sign) pair via sign-magnitude arithmetic instead of running_covariance_step's plain non-negative u32 sums -- the same width/sign gap running_min_max_step_i16 already filled for min/max tracking in this pack.
//! tags: running, covariance, bivariate, stream, accumulate, stats, state, signed, i16, wide, u32, checked, escalate
//! entry: RunningCovarianceI16::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if sum_x_mag, sum_y_mag, or sum_xy_mag would overflow u32
fn i16_mag(v: i16) -> u32 {
    if v < 0i16 { (0u16.wrapping_sub(v as u16)) as u32 } else { v as u16 as u32 }
}
fn i16_neg(v: i16) -> u16 {
    if v < 0i16 { 1u16 } else { 0u16 }
}
struct RunningCovarianceI16 { x: i16, y: i16, count: u32, sum_x_mag: u32, sum_x_neg: u16, sum_y_mag: u32, sum_y_neg: u16, sum_xy_mag: u32, sum_xy_neg: u16 }
impl RunningCovarianceI16 {
    fn run(&mut self) -> u16 {
        let x_mag = i16_mag(self.x);
        let x_neg = i16_neg(self.x);
        let y_mag = i16_mag(self.y);
        let y_neg = i16_neg(self.y);

        // xy = x * y (sign-magnitude product); x_mag,y_mag <= 32768 so the product always
        // fits u32 (max 2^30) -- no overflow check needed for this multiply.
        let xy_mag = x_mag * y_mag;
        let xy_neg = if x_neg == y_neg { 0u16 } else { 1u16 };

        // sum_x += x (the smag_add shape: same-sign adds magnitudes via the checked helper,
        // opposite-sign subtracts the smaller magnitude from the larger and the result's
        // sign follows whichever operand had the larger magnitude).
        let mut new_sum_x_mag = 0u32;
        let mut new_sum_x_neg = 0u16;
        if self.sum_x_neg == x_neg {
            new_sum_x_mag = add_checked_u32(self.sum_x_mag, x_mag);
            new_sum_x_neg = x_neg;
        } else if self.sum_x_mag >= x_mag {
            new_sum_x_mag = self.sum_x_mag - x_mag;
            new_sum_x_neg = if new_sum_x_mag == 0u32 { 0u16 } else { self.sum_x_neg };
        } else {
            new_sum_x_mag = x_mag - self.sum_x_mag;
            new_sum_x_neg = x_neg;
        }
        self.sum_x_mag = new_sum_x_mag;
        self.sum_x_neg = new_sum_x_neg;

        // sum_y += y (the smag_add shape).
        let mut new_sum_y_mag = 0u32;
        let mut new_sum_y_neg = 0u16;
        if self.sum_y_neg == y_neg {
            new_sum_y_mag = add_checked_u32(self.sum_y_mag, y_mag);
            new_sum_y_neg = y_neg;
        } else if self.sum_y_mag >= y_mag {
            new_sum_y_mag = self.sum_y_mag - y_mag;
            new_sum_y_neg = if new_sum_y_mag == 0u32 { 0u16 } else { self.sum_y_neg };
        } else {
            new_sum_y_mag = y_mag - self.sum_y_mag;
            new_sum_y_neg = y_neg;
        }
        self.sum_y_mag = new_sum_y_mag;
        self.sum_y_neg = new_sum_y_neg;

        // sum_xy += x*y (the smag_add shape).
        let mut new_sum_xy_mag = 0u32;
        let mut new_sum_xy_neg = 0u16;
        if self.sum_xy_neg == xy_neg {
            new_sum_xy_mag = add_checked_u32(self.sum_xy_mag, xy_mag);
            new_sum_xy_neg = xy_neg;
        } else if self.sum_xy_mag >= xy_mag {
            new_sum_xy_mag = self.sum_xy_mag - xy_mag;
            new_sum_xy_neg = if new_sum_xy_mag == 0u32 { 0u16 } else { self.sum_xy_neg };
        } else {
            new_sum_xy_mag = xy_mag - self.sum_xy_mag;
            new_sum_xy_neg = xy_neg;
        }
        self.sum_xy_mag = new_sum_xy_mag;
        self.sum_xy_neg = new_sum_xy_neg;

        self.count = self.count + 1u32;
        1u16
    }
}
