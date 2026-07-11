//! Excel-compatible TRUNC(number, [num_digits=0]): truncates a real f32 number toward zero to num_digits decimal places by scaling by 10^num_digits, truncating via .trunc() (the ftrunc kernel, bit-identical to rustc's f32::trunc, which truncates both positive and negative operands toward zero in a single op with no sign-magnitude split needed), then dividing back by the same scale -- deliberately NOT built on f32_to_int_trunc/int_to_f32 (that pair targets an unsigned integer domain and halts 0xFF08 on any negative input, exactly wrong for a function whose whole point is symmetric truncation of negative numbers) nor on a Q8.8-only div_i16(x, 256) shortcut (which only covers the num_digits=0 whole-number case and forces every caller's number through a fixed-point conversion first, the opposite of a broadly reusable real-valued primitive); distinct from ROUND/ROUNDDOWN (which round to the nearest, or away-from-zero, value at num_digits -- this only ever discards, never adjusts, the trailing digits) and from INT (which floors toward negative infinity, so INT(-8.9) = -9 while TRUNC(-8.9) = -8 -- the two disagree on every negative non-integer).
//! tags: excel, trunc, truncate, truncation, decimal-places, num-digits, toward-zero, discard-digits, scale, rescale, f32, float, softfloat, math-trig
//! kernel_bank: on
//! entry: ExcelTrunc::run
//! limits: escalates (halt 0xFF06, out_of_domain) if num_digits > 9 (beyond f32's ~7 decimal digits of mantissa precision the multiply-trunc-divide scale can't recover any additional meaningful digit, and unlike a bounded Newton loop this scale has no self-correcting convergence check, so an unbounded exponent both wastes T-states building 10^num_digits and risks the scale itself running toward Inf for large numbers); escalates (halt 0xFF08, float_domain) if the truncated-and-rescaled result is NaN, or (halt 0xFF07, float_overflow) if it is non-finite (e.g. a non-finite number input, or a number/num_digits combination whose scale pushes the intermediate product past f32's range); negative num_digits (Excel's own convention for truncating to the LEFT of the decimal point, e.g. TRUNC(2312.5, -2) = 2300) is out of scope here -- num_digits is an unsigned u16 field, matching this dialect's no-i16-state-field rule, and a signed num_digits variant would need the same sign-magnitude split docs/excel-mathstat-map.md's own ROUNDDOWN candidate entry already carries; this cell covers Excel's overwhelmingly common non-negative-digit usage only.
struct ExcelTrunc {
    number: f32,
    num_digits: u16,
    result: f32,
}
impl ExcelTrunc {
    fn run(&mut self) -> u16 {
        if self.num_digits > 9u16 {
            halt(0xFF06u16);
        }

        let mut scale = 1.0f32;
        let mut i = 0u16;
        while i < self.num_digits {
            scale = scale * 10.0f32;
            i = i + 1u16;
        }

        let scaled = self.number * scale;
        let truncated = scaled.trunc();
        let result = truncated / scale;

        if result.is_nan() {
            halt(0xFF08u16);
        }
        let fin = result.is_finite();
        if !fin {
            halt(0xFF07u16);
        }

        self.result = result;
        1u16
    }
}
