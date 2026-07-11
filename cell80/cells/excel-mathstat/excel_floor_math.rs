//! Excel-compatible FLOOR.MATH(number, [significance=1], [mode=0]): rounds number DOWN to the nearest multiple of |significance| in the checked-integer domain (number tracked as a num_mag/num_neg sign-magnitude pair, since i16 isn't a valid field type here; significance's own sign is Excel-ignored so sig is taken as a plain nonnegative magnitude with no separate flag) -- for a nonnegative number the case-split collapses to a single floor-division-then-rescale (q = num_mag/sig via div_floor_u32's technique, scaled = q*sig via mul_checked_u32) since mode never affects a positive value; for a negative number, mode=0 (default) rounds the magnitude UP to the next grid line so the value moves AWAY from zero (toward negative infinity, real floor behavior), while mode!=0 rounds the magnitude DOWN so the value moves TOWARD zero instead -- this is the exact mirror image of excel_ceiling_math's own mode convention (CEILING.MATH's default rounds a negative number's magnitude down, toward zero, and its nonzero mode rounds up, away from zero), and distinct from excel_floor_precise (an f32-tier, sign-agnostic floor with no mode flag at all, always toward negative infinity regardless of sign).
//! tags: excel, floor, floor-math, round-down, mode, negative-infinity, away-from-zero, toward-zero, sign-magnitude, significance, multiple, grid, quantize, checked, u32, wide, escalate, math-trig, ceiling-math-mirror
//! entry: ExcelFloorMath::run
//! limits: significance's sign is ignored by Excel so sig is a plain nonnegative magnitude field (no sig_neg flag exists); pass sig=1, mode=0 to reproduce Excel's own omitted-argument defaults (unlike excel_floor_precise's f32 field, a u16 significance has no omitted-vs-explicit ambiguity since 1 is already the natural default value); sig==0 or num_mag==0 both return result_mag=0, result_neg=0 directly, matching Excel's own FLOOR.MATH(number, 0)=0 special case rather than dividing by zero; result_mag is 0 with result_neg forced to 0 whenever the rounded magnitude collapses to zero (no negative zero); escalates (halt 0xFF06, out_of_domain) if num_neg is anything other than 0 or 1; result_mag is u32-width (not u16) since the mode=0 away-from-zero branch can round a u16-bounded magnitude up past 65535 (e.g. num_mag=65535, sig=7 rounds to 65541) -- mul_checked_u32 is still called defensively for every rescale even though a u16-bounded num_mag/sig pair can never actually drive it past u32::MAX.
struct ExcelFloorMath {
    num_mag: u16,
    num_neg: u16,
    sig: u16,
    mode: u16,
    result_mag: u32,
    result_neg: u16,
}
impl ExcelFloorMath {
    fn run(&mut self) -> u16 {
        if self.num_neg > 1u16 {
            halt(0xFF06u16);
        }

        if self.sig == 0u16 || self.num_mag == 0u16 {
            self.result_mag = 0u32;
            self.result_neg = 0u16;
            return 1u16;
        }

        let sig32 = self.sig as u32;
        let num32 = self.num_mag as u32;
        let q = num32 / sig32;

        if self.num_neg == 0u16 {
            // Positive number: floor toward zero and floor toward -infinity are the same
            // direction, so mode has no effect at all (matches CEILING.MATH's own
            // positive-number case, and Excel's documented mode-is-negative-only rule).
            let scaled = mul_checked_u32(q, sig32);
            self.result_mag = scaled;
            self.result_neg = 0u16;
            return 1u16;
        }

        // Negative number: q*sig is the grid line at or below num_mag (toward zero), and
        // (q+1)*sig is the next grid line at or above num_mag (away from zero) whenever
        // num_mag isn't already an exact multiple.
        let qs = mul_checked_u32(q, sig32);
        let r = num32 - qs;

        if self.mode == 0u16 {
            // Default: away from zero (toward -infinity) -- the mirror of
            // excel_ceiling_math's default, which instead rounds toward zero here.
            let mag = if r == 0u32 { num32 } else { mul_checked_u32(q + 1u32, sig32) };
            self.result_mag = mag;
            self.result_neg = 1u16;
        } else {
            // mode != 0: toward zero -- the mirror of excel_ceiling_math's nonzero mode,
            // which instead rounds away from zero here. The remainder is simply dropped,
            // no round-up.
            self.result_mag = qs;
            let neg = if qs == 0u32 { 0u16 } else { 1u16 };
            self.result_neg = neg;
        }
        1u16
    }
}
