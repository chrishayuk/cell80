//! Hyperbolic cotangent coth(x) = cosh(x)/sinh(x) = 1/tanh(x), x != 0.0 -- built from the
//! SAME numerically-stable building block TANH itself uses (a single fexp call at
//! t = e^(-2*|x|), never the naive independent e^x/e^-x pair SINH/COSH each evaluate and
//! add/subtract): unlike SINH/COSH (where a lone Inf never fights a lone 0 under
//! addition or subtraction, so no cancellation trap exists there) reusing that same
//! independent-ep/en shape as a RATIO fails exactly the way TANH's own doc comment
//! already documents -- once x grows past fexp's own overflow threshold (~88.7), e^x
//! alone saturates to +Inf, so cosh's numerator and sinh's denominator BOTH
//! independently collapse to that same +Inf, and Inf/Inf resolves to NaN, a spurious
//! escalation on an input whose true coth is simply the boundary value +/-1.0. This
//! cell instead takes a = |x| and evaluates t = (-2.0*a).exp(), an argument that is
//! never positive, so the kernel only ever underflows gracefully toward 0.0 (never
//! overflows toward Inf): coth(x) = sign(x) * (1 + e^-2a)/(1 - e^-2a) -- the exact
//! reciprocal of TANH's own (1 - e^-2a)/(1 + e^-2a), same t, numerator and denominator
//! swapped, sign re-applied afterward the same way. The one genuine pole sits at
//! x == 0.0 itself (t == 1.0 there, so the denominator 1 - t is exactly 0.0) -- checked
//! upfront directly against x, since (unlike TAN/SEC's periodic poles at multiples of
//! pi/2, only expressible through the computed cos(x)) coth's one and only exclusion is
//! the input's own zero. Distinct from TANH (this cell's own reciprocal) and from the
//! circular COT (cos(x)/sin(x), periodic and unrelated despite the shared name root).
//! tags: coth, hyperbolic-cotangent, cotanh, hyperbolic, reciprocal-tanh, exponential, exp, fexp, pole, asymptote, odd-function, f32, float, softfloat, trig
//! kernel_bank: on
//! entry: CothF32::run
//! accuracy: <= ~4 ulp away from x == 0.0 (one fexp evaluation at <= 1 ulp, combined through a correctly-rounded add, subtract, and divide -- the same error budget TANH's own doc comment already carries, since this cell shares its every arithmetic step but the final reciprocal-of-ratio swap); known limit as |x| grows large: t underflows toward 0.0, so both the numerator (1+t) and denominator (1-t) round toward 1.0 and the ratio toward the correct +/-1.0 asymptote cleanly, with no cancellation; the weakest relative accuracy sits just outside the domain guard below, where t is close to 1.0 and 1 - t suffers the same near-1.0 cancellation gap TANH's own doc comment already flags for its own numerator there.
//! limits: escalates (halt 0xFF08, float_domain) if |x| < 1e-6 -- the pole at x == 0.0 itself, checked directly against the input (coth's one and only domain exclusion) BEFORE t is even computed; escalates (halt 0xFF08, float_domain) if the resulting ratio is itself NaN (only reachable if x itself was already NaN, since a genuine finite nonzero x always keeps t inside [0.0, 1.0) and 1 - t strictly positive, matching TANH's own structural argument), or (halt 0xFF07, float_overflow) if it is otherwise non-finite.
struct CothF32 {
    x: f32,
    result: f32,
}
impl CothF32 {
    fn run(&mut self) -> u16 {
        let ax = self.x.abs();
        if ax < 0.000001f32 {
            halt(0xFF08u16);
        }
        let neg = self.x < 0.0f32;
        let t = (-(ax + ax)).exp();
        let num = 1.0f32 + t;
        let den = 1.0f32 - t;
        let ratio = num / den;
        let r = if neg { -ratio } else { ratio };
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
