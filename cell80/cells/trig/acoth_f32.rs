//! Inverse hyperbolic cotangent acoth(x) = 0.5 * ln((x+1)/(x-1)), defined only for |x| > 1 (inside the open interval -1 <= x <= 1 the ratio (x+1)/(x-1) is either negative -- which fln already maps to NaN -- or, right at the endpoints, a division by zero) -- composed directly from one F2 fln call (the owned transcendental kernel, exposed to this dialect as `.ln()`), the exact mirror-image construction of this pack's own ATANH (0.5 * ln((1+x)/(1-x)), valid only *inside* (-1, 1)): swapping which of the two terms is negated flips the domain from the open interval to its exterior, since acoth(x) == atanh(1/x) for |x| > 1. Distinct from COTH (hyperbolic cotangent, the forward function with no logarithm, defined everywhere x != 0) and from ACOT (arccotangent of a single value, a circular not hyperbolic inverse) -- this pack's fourth cell built on the newly-shipped F2 owned transcendentals.
//! tags: acoth, arccoth, inverse-hyperbolic-cotangent, hyperbolic-arccotangent, hyperbolic, logarithm, ln, transcendental, f32, float, softfloat, trig
//! kernel_bank: on
//! entry: AcothF32::run
//! accuracy: <= ~4 ulp (one fln call at <= 2 ulp, one fadd/fsub pair building the ratio, one fdiv, one fmul by 0.5 -- exact halving via a power-of-two multiply costs no rounding of its own); known limit: as |x| -> infinity the ratio (x+1)/(x-1) crowds toward 1.0 from both sides, the same fln-near-1 relative-accuracy gap ATANH's own accuracy line already documents (there for x -> 0, here for |x| -> infinity), so acoth's relative accuracy is weakest for large |x| even though the absolute error stays tiny there (acoth itself is approaching 0 there too).
//! limits: escalates (halt 0xFF08, float_domain) if |x| <= 1.0 -- the domain boundary itself (x = +/-1 would divide by zero, and |x| < 1 flips the ratio negative, which fln already maps to NaN) is treated as a float_domain condition rather than out_of_domain, an explicit upfront check rather than relying on the NaN/Inf that would otherwise propagate from the ratio; escalates (halt 0xFF08, float_domain) on a NaN result, (halt 0xFF07, float_overflow) on a non-finite one -- both only reachable if x itself was already NaN/non-finite, since a genuine finite x with |x| > 1 always has a finite, non-NaN acoth.
struct AcothF32 {
    x: f32,
    result: f32,
}
impl AcothF32 {
    fn run(&mut self) -> u16 {
        let ax = self.x.abs();
        if ax <= 1.0f32 {
            halt(0xFF08u16);
        }
        let num = self.x + 1.0f32;
        let den = self.x - 1.0f32;
        let ratio = num / den;
        let l = ratio.ln();
        let r = 0.5f32 * l;
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
