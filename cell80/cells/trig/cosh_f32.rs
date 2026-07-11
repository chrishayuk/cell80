//! Hyperbolic cosine cosh(x) = (e^x + e^-x) / 2, composed directly from two independent calls into the shared F2 fexp kernel (e^x via x.exp(), e^-x via (-x).exp() -- the same kernel evaluated at the negated input, not a separate reciprocal/fdiv path) and one exact halving (a power-of-two multiply, no rounding of its own) -- unlike SINH (this pack's other hyperbolic-exponential cell, the odd combination (e^x - e^-x)/2, zero at x == 0.0 and antisymmetric) cosh is even (cosh(-x) == cosh(x) exactly, since swapping x and -x only swaps which fexp call produces which addend) and never below 1.0, and unlike ATANH (built over one fln call, defined only on the open interval (-1, 1)) cosh has no domain restriction at all: every finite x has a defined result. Also distinct from the circular COS (`.cos()`, the fcos kernel, Cody-Waite reduced and bounded to [-1, 1]) despite the shared name root -- cosh is unbounded and grows monotonically for x > 0.
//! tags: hyperbolic, hyperbolic-cosine, cosh, catenary, exponential, exp, fexp, unbounded, even-function, f32, float, softfloat, trig
//! kernel_bank: on
//! entry: CoshF32::run
//! accuracy: <= 2 ulp (two independent fexp evaluations, each <= 1 ulp, combined through one exact-rounded add and one exact halving); unlike SINH, cosh's addend combination is a sum, not a difference, so there is no catastrophic-cancellation gap near x == 0.0 -- the two ~1.0 terms there simply add to ~2.0 and halve back to 1.0 cleanly.
//! limits: cosh has no restricted domain -- every finite x has a defined result (the smaller of the two fexp terms always underflows toward 0.0 well before the larger one overflows, so the pair never fights each other); escalates (halt 0xFF08, float_domain) if the computed result is NaN (only reachable if x itself was already NaN), (halt 0xFF07, float_overflow) if it is non-finite -- reachable once |x| grows enough that e^|x| itself overflows f32 (a bit past x ~= +/-88.7, fexp's own overflow threshold), matching cosh's own real unbounded growth rather than any artificial cutoff.
struct CoshF32 {
    x: f32,
    result: f32,
}
impl CoshF32 {
    fn run(&mut self) -> u16 {
        let ep = self.x.exp();
        let en = (-self.x).exp();
        let sum = ep + en;
        let r = sum * 0.5f32;
        if r.is_nan() {
            halt(0xFF08u16);
        }
        let fin = r.is_finite();
        if !fin {
            halt(0xFF07u16);
        }
        self.result = r;
        1u16
    }
}
