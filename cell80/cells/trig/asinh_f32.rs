//! Inverse hyperbolic sine (hyperbolic arcsine) of a real value x: asinh(x) = ln(x + sqrt(x*x + 1)), composed directly from the rustz80 fsqrt (F0, correctly rounded) and fln (F2, <= 1 ulp) kernels -- defined for ALL real x with no domain restriction (unlike ACOSH, which needs x >= 1, or ASIN/ACOS, which are bounded to [-1, 1]), since x*x + 1 is always >= 1 (sqrt's argument is always real) and sqrt(x*x + 1) always strictly exceeds |x| (so x + sqrt(x*x + 1) is always strictly positive, keeping fln's argument always in-domain algebraically) -- the only escalation surface left is a non-finite outcome for extreme |x|, matching the physics/excel-financial pack's finite-result convention rather than a domain check.
//! tags: trig, hyperbolic, hyperbolic-sine, inverse-hyperbolic-sine, asinh, arcsinh, inverse-hyperbolic-function, logarithm, ln, sqrt, transcendental, f32, float, softfloat
//! entry: AsinhF32::run
//! kernel_bank: on
//! accuracy: <= 1 ulp (fsqrt and the fadd feeding it are correctly-rounded per rustz80's F0 kernel family; the single fln call carries the F2 family's own <= 1 ulp bound, which dominates and sets the whole composition's error)
//! limits: no domain restriction (x*x + 1 > 0 and x + sqrt(x*x + 1) > 0 for every real x, algebraically, so no halt(0xFF06) case exists); escalates (halt 0xFF08, float_domain) on a NaN result (e.g. a NaN input x), (halt 0xFF07, float_overflow) on a non-finite one (x*x overflowing to +Inf for very large |x|)
struct AsinhF32 {
    x: f32,
    asinh: f32,
}
impl AsinhF32 {
    fn run(&mut self) -> u16 {
        let x2 = self.x * self.x;
        let under_sqrt = x2 + 1.0f32;
        let s = under_sqrt.sqrt();
        let arg = self.x + s;
        let r = arg.ln();
        if r.is_nan() {
            halt(0xFF08u16);
        }
        let fin = r.is_finite();
        if !fin {
            halt(0xFF07u16);
        }
        self.asinh = r;
        1u16
    }
}
