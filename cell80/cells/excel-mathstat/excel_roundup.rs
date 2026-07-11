//! Excel-compatible ROUNDUP(number, num_digits) in cell80's checked-int tier: number is tracked as a num_mag/num_neg sign-magnitude pair and num_digits as a digits_mag/digits_neg pair (ExcelRound's own num_digits split, since i16 isn't a valid field type here), but unlike ROUND's f32 scale-round-rescale, a checked-int-tier number has no fractional part at all, so a nonnegative num_digits (digits_neg == 0) is always a no-op passthrough (Excel's own behavior on an already-whole number) and only a negative num_digits (digits_neg == 1, rounding to a tens/hundreds/etc. place) does any work: pow10 = 10^digits_mag is built by a bounded repeated-multiply loop through mul_checked_u32 (dollarfr's digit-count-to-pow10 technique, escalating rather than wrapping past u32::MAX), then div_ceil_u32's own q+1-if-remainder technique rounds the magnitude UP to the next multiple of pow10 and never down -- the one change that makes this ROUNDUP rather than its truncating sibling ROUNDDOWN (same num_mag/digits_mag scale, opposite rounding direction) -- and distinct from MROUND/CEILING.MATH (round to an arbitrary caller-supplied significance/step with their own sign-mismatch or mode-flag rules, not a fixed decimal-digit-count power of ten).
//! tags: excel, roundup, round-up, round, ceiling, away-from-zero, magnitude, sign-magnitude, signed, digits, num-digits, digit-count, power-of-ten, scale, checked, escalate, wide, u32, math-trig
//! entry: ExcelRoundUp::run
//! limits: escalates (halt 0xFF06, out_of_domain) if num_neg or digits_neg is anything other than 0 or 1; num_mag == 0 always short-circuits to result_mag=0, result_neg=0 regardless of digits_mag/digits_neg/num_neg's own values, matching Excel's own ROUNDUP(0, n) = 0; a nonnegative num_digits (digits_neg == 0) never touches the pow10 loop at all, so no digits_mag bound applies to that branch -- only a negative num_digits (digits_neg == 1) does, where the repeated-multiply loop escalates (halt 0xFF05, needs_wider_math) the moment 10^digits_mag would exceed u32::MAX (digits_mag >= 10), and the final rescale multiply escalates the same way if rounding the magnitude up would overflow u32.
struct ExcelRoundUp {
    num_mag: u16,
    num_neg: u16,
    digits_mag: u16,
    digits_neg: u16,
    result_mag: u32,
    result_neg: u16,
}
impl ExcelRoundUp {
    fn run(&mut self) -> u16 {
        if self.num_neg > 1u16 || self.digits_neg > 1u16 {
            halt(0xFF06u16);
        }

        if self.num_mag == 0u16 {
            self.result_mag = 0u32;
            self.result_neg = 0u16;
            return 1u16;
        }

        if self.digits_neg == 0u16 {
            // num_digits >= 0: a checked-int-tier number has no fractional part, so
            // rounding UP to a decimal place at or right of the ones digit never has
            // anything to bump -- Excel's own no-op behavior on a whole number.
            self.result_mag = self.num_mag as u32;
            self.result_neg = self.num_neg;
            return 1u16;
        }

        // num_digits < 0: build 10^digits_mag via a bounded repeated-multiply loop
        // (dollarfr's own digit-count-to-pow10 technique), using mul_checked_u32 so an
        // unrealistically large digits_mag escalates rather than wrapping.
        let mut pow10 = 1u32;
        let mut i = 0u16;
        while i < self.digits_mag {
            pow10 = mul_checked_u32(pow10, 10u32);
            i = i + 1u16;
        }

        let num32 = self.num_mag as u32;
        let q = num32 / pow10;
        let r = num32 % pow10;
        // div_ceil_u32's own q+1-if-remainder technique: ceiling in magnitude, never
        // truncating -- the one line that makes this ROUNDUP instead of ROUNDDOWN.
        let rounded_q = if r != 0u32 { q + 1u32 } else { q };
        let scaled = mul_checked_u32(rounded_q, pow10);

        self.result_mag = scaled;
        self.result_neg = self.num_neg;
        1u16
    }
}
