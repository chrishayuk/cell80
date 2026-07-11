//! Excel-compatible FLOOR.PRECISE(number, [significance]): rounds number DOWN to the nearest multiple of |significance|, always toward negative infinity regardless of either operand's sign -- computed as ffloor(number / |significance|) * |significance| via plain f32 divide/floor/multiply (all routing through the shipped IEEE binary32 kernels, ffloor being cell80's round-toward-negative-infinity primitive), distinct from checked-arithmetic's snap_down/snap_down_u32 (floor-to-multiple over an unsigned u16/u32-only domain, no negative numbers, no fractional operands, no omittable default) and from div_floor_u32 (a bare integer division, not a rescale-then-floor-then-rescale grid snap), and the unconditional-direction mirror of CEILING.PRECISE (same sign-agnostic |significance|, opposite rounding direction) rather than FLOOR.MATH (which instead exposes a mode flag that changes which way negative numbers round); the omitted-vs-explicit-significance distinction a bare f32 field can't represent by itself is resolved with an explicit sig_omitted flag, documented in limits below.
//! tags: excel, floor, floor-precise, round-down, floor-to-multiple, negative-infinity, significance, sign-agnostic, multiple, grid, quantize, f32, float, softfloat, math-trig, ceiling-precise-mirror
//! kernel_bank: on
//! entry: ExcelFloorPrecise::run
//! limits: significance is omittable -- when sig_omitted != 0, the significance field's own value is ignored entirely and Excel's own default of 1.0 is used instead; when sig_omitted == 0 (significance passed explicitly) and significance == 0.0, the result is 0.0 directly, matching Excel's own FLOOR.PRECISE(number, 0) = 0 rather than dividing by zero; escalates (halt 0xFF08, float_domain) if the floored quotient or the final result is NaN; escalates (halt 0xFF07, float_overflow) if either is non-finite (e.g. from a non-finite number or an extreme significance).
struct ExcelFloorPrecise {
    number: f32,
    significance: f32,
    sig_omitted: u16,
    result: f32,
}
impl ExcelFloorPrecise {
    fn run(&mut self) -> u16 {
        let mut sig = self.significance;
        if self.sig_omitted != 0u16 {
            sig = 1.0f32;
        }

        if sig == 0.0f32 {
            self.result = 0.0f32;
            return 1u16;
        }

        let sig_mag = sig.abs();
        let q = self.number / sig_mag;
        let qf = q.floor();
        if qf.is_nan() {
            halt(0xFF08u16);
        }
        let qf_fin = qf.is_finite();
        if !qf_fin {
            halt(0xFF07u16);
        }

        let result = qf * sig_mag;
        if result.is_nan() {
            halt(0xFF08u16);
        }
        let result_fin = result.is_finite();
        if !result_fin {
            halt(0xFF07u16);
        }

        self.result = result;
        1u16
    }
}
