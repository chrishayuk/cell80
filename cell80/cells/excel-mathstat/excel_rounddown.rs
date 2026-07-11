//! Excel-compatible ROUNDDOWN(number, num_digits): truncates number toward zero at num_digits decimal places, unconditionally -- never rounds up even when the discarded digit is 5 or higher, unlike ROUND's ties-away-from-zero tie rule -- built at the checked-int tier via the same i16 sign-magnitude convention div_i16/mul_i16 use (number split into mag/neg, num_digits split into digits_mag/digits_neg, since neither is a valid signed state field on its own): a positive or zero num_digits (digits_neg == 0) is an unconditional pass-through here, since this integer-only domain already has zero fractional digits to drop (a real behavioral consequence of the checked-int tier, not present in ROUND's f32 tier); a negative num_digits (digits_neg == 1) builds pow10 = 10^digits_mag via a checked repeated-multiply loop through mul_checked_u32 (escalating past ~9 digits, since 10^10 already overflows u32), floor-divides the magnitude by pow10 and multiplies back to truncate it down to that grid, then reapplies the original sign -- the opposite-direction mirror of ROUNDUP (same scale-then-truncate shape, ceiling away from zero instead) and distinct from excel_floor_precise/snap_down_u32 (an unsigned or sign-agnostic floor toward negative infinity to an arbitrary significance, not a truncate-toward-zero to a decimal-place power of ten) and from TRUNC (a fixed Q8.8 scale-by-256 special case, not a variable digit-count power-of-ten scale).
//! tags: excel, rounddown, round-down, truncate, truncation, truncate-toward-zero, unconditional, decimal-places, num-digits, negative-digits, power-of-ten, scale, sign-magnitude, i16, checked, integer, checked-int, math-trig
//! entry: ExcelRounddown::run
//! limits: escalates (halt 0xFF06, out_of_domain) if neg or digits_neg is anything other than 0 or 1; escalates (halt 0xFF05, needs_wider_math) if building pow10 = 10^digits_mag overflows u32 (digits_mag >= 10 -- in practice any digits_mag >= 5 already truncates any u16 magnitude to 0 well before this boundary is reached); a positive or zero num_digits is always a no-op in this checked-int tier (no fractional digits exist to drop); result_neg is canonicalized to 0 whenever result_mag == 0, matching smag_mul/smag_div's own no-negative-zero convention.
struct ExcelRounddown {
    mag: u16,
    neg: u16,
    digits_mag: u16,
    digits_neg: u16,
    result_mag: u16,
    result_neg: u16,
}
impl ExcelRounddown {
    fn run(&mut self) -> u16 {
        if self.neg > 1u16 || self.digits_neg > 1u16 {
            halt(0xFF06u16);
        }

        if self.digits_neg == 0u16 {
            // num_digits >= 0: this checked-int tier already represents `number`
            // with zero fractional digits, so there is nothing to drop -- an
            // unconditional pass-through, unlike ROUND's f32 tier where a positive
            // num_digits still meaningfully scales a genuinely fractional value.
            let pass_neg = if self.mag == 0u16 { 0u16 } else { self.neg };
            self.result_mag = self.mag;
            self.result_neg = pass_neg;
            return 1u16;
        }

        // num_digits < 0: truncate the magnitude down to the nearest 10^digits_mag,
        // always toward zero -- operate on the magnitude and reapply the sign
        // afterward (never truncate the signed value directly), the same technique
        // div_i16/mul_i16 use for their own signed arithmetic.
        let mut pow10 = 1u32;
        let mut i = 0u16;
        while i < self.digits_mag {
            pow10 = mul_checked_u32(pow10, 10u32);
            i = i + 1u16;
        }

        let mag32 = self.mag as u32;
        let scaled = mag32 / pow10;
        let result_mag32 = scaled * pow10;
        let result_mag = result_mag32 as u16;

        let result_neg = if result_mag == 0u16 { 0u16 } else { self.neg };

        self.result_mag = result_mag;
        self.result_neg = result_neg;
        1u16
    }
}
