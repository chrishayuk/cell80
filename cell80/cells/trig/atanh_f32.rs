//! Inverse hyperbolic tangent atanh(x) = 0.5 * ln((1+x)/(1-x)), defined only on the open interval -1 < x < 1 (the ratio (1+x)/(1-x) is positive there and grows without bound as x approaches either endpoint, matching atanh's own asymptotes to -Inf/+Inf) -- composed directly from one F2 fln call (the owned transcendental kernel, exposed to this dialect as `.ln()`) rather than any series expansion, since the closed form is already exact apart from the fln kernel's own rounding. Distinct from the forward TANH (hyperbolic tangent, no logarithm at all) and from plain ATAN (arctangent of a single ratio via fatan2, a circular not hyperbolic inverse) -- this pack's first cell built on the newly-shipped F2 owned transcendentals.
//! tags: atanh, arctanh, inverse-hyperbolic-tangent, hyperbolic-arctangent, fisher-transformation, hyperbolic, logarithm, ln, transcendental, f32, float, softfloat, trig
//! kernel_bank: on
//! entry: AtanhF32::run
//! accuracy: <= ~4 ulp (one fln call at <= 2 ulp, one fadd/fsub pair building the ratio, one fdiv, one fmul by 0.5 -- exact halving via a power-of-two multiply costs no rounding of its own); known limit: as x -> 0 the ratio (1+x)/(1-x) crowds toward 1.0, the same fln-near-1 relative-accuracy gap excel_pduration.rs's own accuracy line already documents for ln(1+rate) at small rate, so atanh's relative accuracy is weakest for small |x| even though the absolute error stays tiny there.
//! limits: escalates (halt 0xFF08, float_domain) if |x| >= 1.0 -- the open-interval domain boundary itself (x = +/-1 would divide by zero or take ln of a non-positive ratio, and |x| > 1 flips the ratio negative, which fln already maps to NaN) is treated as a float_domain condition rather than out_of_domain, an explicit upfront check rather than relying on the NaN/Inf that would otherwise propagate from the ratio; escalates (halt 0xFF08, float_domain) on a NaN result, (halt 0xFF07, float_overflow) on a non-finite one -- both only reachable if x itself was already NaN/non-finite, since a genuine finite x strictly inside (-1, 1) always has a finite, non-NaN atanh.
struct AtanhF32 {
    x: f32,
    result: f32,
}
impl AtanhF32 {
    fn run(&mut self) -> u16 {
        let ax = self.x.abs();
        if ax >= 1.0f32 {
            halt(0xFF08u16);
        }
        let num = 1.0f32 + self.x;
        let den = 1.0f32 - self.x;
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
