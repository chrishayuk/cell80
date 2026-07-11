//! Excel-compatible CEILING.PRECISE(number, [significance]): rounds number UP toward
//! +infinity to the nearest multiple of significance's magnitude, discarding
//! significance's own sign entirely -- unlike this same pack's CEILING.MATH sibling
//! (a sign-aware mode flag that flips negative-number direction), there is no mode
//! argument here at all: direction is fixed at +infinity regardless of sign, the
//! plain textbook ceil(x/s) = -floor(-x/s) identity. number >= 0 snaps up via
//! snap_up_u32's own technique; number < 0 flips to -snap_down_u32(|number|,
//! |significance|), using the sign-magnitude (mag, neg) convention smag_add/smag_mul
//! already establish in the checked-arithmetic pack; a zero significance (or a zero
//! number) short-circuits straight to a positive zero before either branch runs,
//! matching Excel's own CEILING.PRECISE(number, 0) = 0 convention.
//! tags: excel, ceiling, ceiling-precise, round-up, snap, grid, multiple, sign, sign-magnitude, checked-int, u32, wide, math-trig
//! entry: ExcelCeilingPrecise::run
//! limits: escalates (halt 0xFF06, out_of_domain) if number_neg is anything other than 0 or 1; escalates (halt 0xFF05, needs_wider_math) if the ceiling-branch scale-back multiply (quotient * sig) would exceed u32::MAX (the same overflow guard snap_up_u32 already carries -- the floor branch never needs it, since flooring a magnitude down can never grow past it); significance == 0 or number == 0 returns a positive-signed 0, matching Excel's own zero-significance convention.
struct ExcelCeilingPrecise {
    number_mag: u32,
    number_neg: u16,
    sig: u32,
    result_mag: u32,
    result_neg: u16,
}
impl ExcelCeilingPrecise {
    fn run(&mut self) -> u16 {
        if self.number_neg > 1u16 {
            halt(0xFF06u16);
        }

        if self.sig == 0u32 || self.number_mag == 0u32 {
            self.result_mag = 0u32;
            self.result_neg = 0u16;
            return 1u16;
        }

        if self.number_neg == 0u16 {
            // number >= 0: snap_up_u32's own ceiling-to-grid technique directly.
            let q = (self.number_mag - 1u32) / self.sig + 1u32;
            let r = mul_checked_u32(q, self.sig);
            self.result_mag = r;
            self.result_neg = 0u16;
        } else {
            // number < 0: -snap_down_u32(|number|, sig) -- flooring a magnitude can
            // only shrink it, so no overflow-checked multiply is needed here.
            let q = self.number_mag / self.sig;
            let r = q * self.sig;
            let neg = if r == 0u32 { 0u16 } else { 1u16 };
            self.result_mag = r;
            self.result_neg = neg;
        }
        1u16
    }
}
