//! Excel-compatible ODD(number): rounds a number UP, away from zero, to the nearest odd integer (ODD(2)=3, ODD(-2)=-3, ODD(0)=1) -- computed in one step as 2*ceil((|number|-1)/2)+1, a single fceil call on the magnitude (bit-identical to rustc) that lands on the next odd integer at or above |number| whether |number| is itself odd, even, or fractional, with no separate even/odd branch-and-bump needed, then the caller's original sign is reapplied to the magnitude result; the odd-target counterpart to EVEN (checked-int tier: abs_i16 -> round-up-to-even -> reapply sign) and distinct from CEILING.MATH/MROUND (round to a multiple of an arbitrary caller-supplied significance/step, not to odd parity specifically) and from ROUNDUP (rounds away from zero to N decimal digits, no parity constraint at all).
//! tags: excel, odd, round-up, round-away-from-zero, nearest-odd, ceiling, parity, f32, float, softfloat, math-trig
//! kernel_bank: on
//! entry: ExcelOdd::run
//! limits: a NaN or non-finite `number` propagates through abs/ceil arithmetic exactly as IEEE 754 requires, so it is caught by the same final output check rather than a separate upfront one: escalates (halt 0xFF08, float_domain) if the computed result is NaN; escalates (halt 0xFF07, float_overflow) if the computed result is non-finite; ODD(0) returns 1 (Excel's own documented convention -- zero has no sign to preserve, so the magnitude bump to the next odd integer is returned unsigned-positive)
struct ExcelOdd {
    number: f32,
    result: f32,
}
impl ExcelOdd {
    fn run(&mut self) -> u16 {
        let mag = self.number.abs();
        let half = (mag - 1.0f32) * 0.5f32;
        let half_up = half.ceil();
        let mag_odd = 2.0f32 * half_up + 1.0f32;

        let signed = if self.number < 0.0f32 { -mag_odd } else { mag_odd };

        if signed.is_nan() {
            halt(0xFF08u16);
        }
        let out_fin = signed.is_finite();
        if !out_fin {
            halt(0xFF07u16);
        }

        self.result = signed;
        1u16
    }
}
