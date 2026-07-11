//! Excel-compatible SQRT(number): the non-negative square root of a number, taken by calling .sqrt() directly on the f32 field so the result routes straight through the shipped fsqrt kernel (correctly rounded, bit-identical to rustc) -- deliberately NOT built on top of nth_root_f32's general Newton-Raphson Nth-root solver, since n=2 already has its own native single-instruction kernel and looping a bounded refinement toward a value the hardware already produces exactly in one step would be needless indirection.
//! tags: excel, sqrt, square-root, root, radical, non-negative, fsqrt, f32, float, softfloat, math-trig
//! entry: ExcelSqrt::run
//! limits: escalates (halt 0xFF06, out_of_domain) if number < 0.0 -- Excel's own #NUM! convention for a negative radicand (no complex-number result is ever produced); escalates (halt 0xFF08, float_domain) if the computed result is NaN, (halt 0xFF07, float_overflow) if it is non-finite -- both only reachable if number itself was already NaN/non-finite, since a genuinely non-negative finite input always has a finite, non-NaN square root.
struct ExcelSqrt {
    number: f32,
    result: f32,
}
impl ExcelSqrt {
    fn run(&mut self) -> u16 {
        if self.number < 0.0f32 {
            halt(0xFF06u16);
        }
        let r = self.number.sqrt();
        if r.is_nan() {
            halt(0xFF08u16);
        }
        let r_fin = r.is_finite();
        if !r_fin {
            halt(0xFF07u16);
        }
        self.result = r;
        1u16
    }
}
