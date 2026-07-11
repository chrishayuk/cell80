//! Signed sibling of accumulate_step/accumulate_step_u32: running signed sum (tracked as a sign-magnitude pair, same-sign combines via add, opposite-sign combines via subtract-the-smaller-from-the-larger) plus count over a stream of i16 values, checked on magnitude overflow via add_checked_u32 -- neither existing accumulate_step accepts signed input.
//! tags: running, sum, count, accumulate, stream, stats, mean, average, state, signed, i16, wide, u32, checked, escalate
//! entry: AccumulateI16::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if the running sum's magnitude overflows u32
fn i16_mag(v: i16) -> u32 {
    if v < 0i16 { (0u16.wrapping_sub(v as u16)) as u32 } else { v as u16 as u32 }
}
fn i16_neg(v: i16) -> u16 {
    if v < 0i16 { 1u16 } else { 0u16 }
}
struct AccumulateI16 { value: i16, sum_mag: u32, sum_neg: u16, count: u32 }
impl AccumulateI16 {
    fn run(&mut self) -> u16 {
        let v_mag = i16_mag(self.value);
        let v_neg = i16_neg(self.value);

        // sum = sum + value, via sign-magnitude: same sign is a checked add of magnitudes,
        // opposite sign is a subtract of the smaller magnitude from the larger, with the
        // result's sign following whichever operand had the larger magnitude (forced to
        // nonnegative if the result magnitude lands on zero).
        let mut new_mag = 0u32;
        let mut new_neg = 0u16;
        if self.sum_neg == v_neg {
            new_mag = add_checked_u32(self.sum_mag, v_mag);
            new_neg = v_neg;
        } else if self.sum_mag >= v_mag {
            new_mag = self.sum_mag - v_mag;
            new_neg = if new_mag == 0u32 { 0u16 } else { self.sum_neg };
        } else {
            new_mag = v_mag - self.sum_mag;
            new_neg = if new_mag == 0u32 { 0u16 } else { v_neg };
        }
        self.sum_mag = new_mag;
        self.sum_neg = new_neg;
        self.count = self.count + 1u32;
        1u16
    }
}
