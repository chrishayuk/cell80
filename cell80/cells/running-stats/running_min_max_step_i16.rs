//! Signed sibling of running_min_max_step over a stream of i16 values: updates min/max (self-initializing on the first call via `seen`), returns the current range (max - min) as u16 via sign-magnitude subtraction (the abs_diff_i16 shape), since max=i16::MAX and min=i16::MIN would overflow i16 by one before a native subtraction could produce the range.
//! tags: running, min, max, range, stream, stats, tracker, state, signed, i16
//! entry: RunningMinMaxI16::run
fn i16_mag(v: i16) -> u32 {
    if v < 0i16 { (0u16.wrapping_sub(v as u16)) as u32 } else { v as u16 as u32 }
}
fn i16_neg(v: i16) -> u16 {
    if v < 0i16 { 1u16 } else { 0u16 }
}
struct RunningMinMaxI16 { value: i16, min: i16, max: i16, seen: u16 }
impl RunningMinMaxI16 {
    fn run(&mut self) -> u16 {
        if self.seen == 0u16 {
            self.min = self.value;
            self.max = self.value;
            self.seen = 1u16;
        } else {
            // value < min / value > max, decided via sign-magnitude rather than a native
            // i16 compare of two independently-tracked signed fields: negative beats any
            // nonnegative for "less", same-sign ties break on magnitude (inverted for the
            // negative side, since -100 has a bigger magnitude than -1 but is the smaller
            // value).
            let v_neg = i16_neg(self.value);
            let v_mag = i16_mag(self.value);
            let min_neg = i16_neg(self.min);
            let min_mag = i16_mag(self.min);
            let max_neg = i16_neg(self.max);
            let max_mag = i16_mag(self.max);

            let is_less = if v_neg != min_neg { (v_neg == 1u16) as u16 }
                else if v_neg == 1u16 { (v_mag > min_mag) as u16 }
                else { (v_mag < min_mag) as u16 };
            let is_greater = if v_neg != max_neg { (v_neg == 0u16) as u16 }
                else if v_neg == 1u16 { (v_mag < max_mag) as u16 }
                else { (v_mag > max_mag) as u16 };

            if is_less == 1u16 { self.min = self.value; }
            if is_greater == 1u16 { self.max = self.value; }
        }

        // range = max - min, tracked via sign-magnitude (the abs_diff_i16 shape) since
        // max=i16::MAX, min=i16::MIN would overflow i16 by one before a native subtract.
        let max_mag2 = i16_mag(self.max);
        let max_neg2 = i16_neg(self.max);
        let min_mag2 = i16_mag(self.min);
        let min_neg_f2 = 1u16 - i16_neg(self.min);

        let range_mag = if max_neg2 == min_neg_f2 { add_checked_u32(max_mag2, min_mag2) }
            else if max_mag2 >= min_mag2 { max_mag2 - min_mag2 }
            else { min_mag2 - max_mag2 };
        range_mag as u16
    }
}
