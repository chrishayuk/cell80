//! Hyperbolic secant sech(x) = 1/cosh(x) = 2/(e^x + e^-x), composed directly over two independent calls into the shared F2 fexp kernel (e^x via x.exp(), e^-x via (-x).exp() -- the same kernel evaluated at the negated input, not a separate reciprocal/fdiv path bolted onto COSH itself) followed by one exact halving and one ordinary f32 divide -- no dedicated secant-hyperbolic minimax polynomial is used, since cosh's own closed form already gives an exact reciprocal path. Unlike CSCH (this pack's other hyperbolic-reciprocal cell, 1/sinh(x), with a true pole at x == 0.0 since sinh(0.0) == 0.0 exactly) sech has no pole at all: cosh(x) is never zero (its minimum is exactly 1.0, reached at x == 0.0), so no upfront near-zero guard is needed before the divide. sech is even (sech(-x) == sech(x) exactly, since swapping x and -x only swaps which fexp call produces which addend) and bounded to (0.0, 1.0], peaking at exactly 1.0 when x == 0.0 and vanishing toward 0.0 as |x| grows without ever reaching it. Distinct from COSH (this cell's own reciprocal target, not the hyperbolic-cosine value itself), from CSCH (the other hyperbolic reciprocal, carrying a genuine pole this cell does not share), and from SEC (the circular reciprocal of fcos, periodic with poles at every odd multiple of pi/2, rather than pole-free and monotonically vanishing).
//! tags: sech, secant-hyperbolic, hyperbolic-secant, reciprocal-hyperbolic-cosine, reciprocal-of-cosh, hyperbolic, exponential, exp, fexp, bounded, even-function, f32, float, softfloat, trig
//! kernel_bank: on
//! entry: SechF32::run
//! accuracy: <= 2 ulp (two independent fexp evaluations, each <= 1 ulp, combined through one exact-rounded add, one exact halving, and one f32 divide); no catastrophic-cancellation gap anywhere (the denominator is always a sum of two positive terms, never a difference), so accuracy stays uniform across the whole domain rather than degrading near any particular x.
//! limits: sech has no restricted domain and no pole -- cosh(x) is never zero, so the divide is always well-defined for finite x; escalates (halt 0xFF08, float_domain) if the computed result is NaN (only reachable if x itself was already NaN), (halt 0xFF07, float_overflow) if it is non-finite -- not reachable through ordinary overflow the way COSH's own result can overflow, since sech is bounded to (0.0, 1.0], but checked anyway for consistency with the rest of the pack's escalation convention and to catch any unexpected non-finite propagation from the underlying fexp kernel.
struct SechF32 {
    x: f32,
    result: f32,
}
impl SechF32 {
    fn run(&mut self) -> u16 {
        let ep = self.x.exp();
        let en = (-self.x).exp();
        let sum = ep + en;
        let ch = sum * 0.5f32;
        let r = 1.0f32 / ch;
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
