//! Excel CEILING.MATH(number, [significance=1], [mode=0]): rounds number UP to the nearest multiple of significance's magnitude, but for a NEGATIVE number the mode argument decides which way "up" points -- mode=0 (Excel's own default) rounds toward zero (the smaller-magnitude multiple), any nonzero mode rounds away from zero (the larger-magnitude multiple) -- number arrives pre-split sign-magnitude (number_mag, number_neg) since i16 isn't a valid state field, and the two magnitude-only branches directly inline snap_up_u32's ceiling-to-grid technique (positive numbers, and negative numbers when mode!=0) or snap_down_u32's floor-to-grid technique (negative numbers when mode==0), rather than calling either cell (cells can't call each other) -- distinct from plain CEILING/snap_up_u32 (always rounds away from zero, no mode argument or negative-number direction choice exists at all) and from FLOOR.MATH (the identical sign/mode-flag shape, but its positive-number branch rounds DOWN, not up, and its mode=0 negative branch rounds away from zero rather than toward it).
//! tags: excel, ceiling, ceiling-math, round-up, mode-flag, sign-magnitude, negative-number, direction, toward-zero, away-from-zero, snap-up, snap-down, checked, wide, u32, escalate
//! entry: ExcelCeilingMath::run
//! limits: significance==0 returns 0 (Excel's own #DIV/0! collapsed to a zero result here, per the assignment's convention, rather than an escalation); escalates (halt 0xFF06, out_of_domain) if number_neg is anything other than 0 or 1; escalates (halt 0xFF05, needs_wider_math) if the ceiling scale-back multiply (quotient * significance) would exceed u32::MAX
struct ExcelCeilingMath {
    number_mag: u32,
    number_neg: u16,
    significance: u32,
    mode: u16,
    result_mag: u32,
    result_neg: u16,
}
impl ExcelCeilingMath {
    fn run(&mut self) -> u16 {
        if self.number_neg > 1u16 {
            halt(0xFF06u16);
        }
        if self.significance == 0u32 {
            self.result_mag = 0u32;
            self.result_neg = 0u16;
            return 1u16;
        }

        if self.number_neg == 0u16 {
            // Positive number: always rounds away from zero (up), regardless of mode --
            // snap_up_u32's own ceiling-to-grid technique, inlined against the magnitude.
            let r = if self.number_mag == 0u32 {
                0u32
            } else {
                let q = (self.number_mag - 1u32) / self.significance + 1u32;
                mul_checked_u32(q, self.significance)
            };
            self.result_mag = r;
            self.result_neg = 0u16;
        } else if self.mode == 0u16 {
            // Negative number, mode 0 (default): rounds TOWARD zero -- the smaller-magnitude
            // multiple, found by flooring the magnitude to grid (snap_down_u32's technique).
            let r = (self.number_mag / self.significance) * self.significance;
            self.result_mag = r;
            let n = if r == 0u32 { 0u16 } else { 1u16 };
            self.result_neg = n;
        } else {
            // Negative number, mode != 0: rounds AWAY from zero -- the larger-magnitude
            // multiple, found by ceiling the magnitude to grid (snap_up_u32's technique).
            let r = if self.number_mag == 0u32 {
                0u32
            } else {
                let q = (self.number_mag - 1u32) / self.significance + 1u32;
                mul_checked_u32(q, self.significance)
            };
            self.result_mag = r;
            let n = if r == 0u32 { 0u16 } else { 1u16 };
            self.result_neg = n;
        }
        1u16
    }
}
