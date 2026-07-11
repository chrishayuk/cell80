//! Hyperbolic tangent tanh(x) = sinh(x)/cosh(x), algebraically (e^x - e^-x)/(e^x + e^-x) -- but NOT composed that naive way. Unlike SINH/COSH (this pack's other hyperbolic-exponential cells, which independently evaluate e^x and e^-x via two separate fexp calls and simply add or subtract them -- safe because a lone Inf never fights a lone 0 under addition or subtraction), TANH cannot reuse that same independent-ep/en shape as a ratio: once x grows past fexp's own overflow threshold (~88.7), e^x alone saturates to +Inf, so sinh's would-be numerator (e^x - e^-x) and cosh's would-be denominator (e^x + e^-x) BOTH independently collapse to that same +Inf, and Inf/Inf resolves to NaN -- a spurious escalation on an input whose true tanh is simply the boundary value 1.0. This cell avoids that trap entirely: it takes a = |x| and evaluates a single (-2.0*a).exp() instead, an argument that is never positive, so the kernel only ever underflows gracefully toward 0.0 (never overflows toward Inf) -- tanh(x) = sign(x) * (1 - e^-2a)/(1 + e^-2a), the textbook numerically-stable shape, with the sign re-applied afterward. Distinct from ATANH (this pack's other hyperbolic cell, built over one fln call rather than fexp, and domain-restricted to the open interval (-1, 1), whereas TANH accepts every real x) and from TAN (the circular tangent, sin/cos through fsin/fcos, periodic and asymptotic rather than saturating).
//! tags: tanh, hyperbolic-tangent, hyperbolic, sigmoid-family, squashing-function, activation-function, saturating, exponential, exp, fexp, bounded, odd-function, f32, float, softfloat, trig
//! kernel_bank: on
//! entry: TanhF32::run
//! accuracy: <= ~4 ulp away from x == 0.0 (one fexp evaluation at <= 1 ulp, combined through a correctly-rounded add, subtract, and divide); known limit near x == 0.0: e^-2a rounds to ~1.0 there, so the numerator 1 - e^-2a suffers the same catastrophic-cancellation-against-1.0 gap excel_nper.rs/excel_pduration.rs/SINH already flag for their own small-argument corners -- the absolute error floor stays near one f32 ulp of 1.0 (~2^-24) even as the true result shrinks toward 0.0, so relative accuracy is weakest for small |x|, not hidden.
//! limits: tanh has no restricted domain -- every real x (including +/-infinity, which resolve to the +/-1.0 asymptote exactly, since (-2.0*a).exp() underflows cleanly to 0.0 for large a rather than ever driving the kernel toward its overflow branch) is defined; escalates (halt 0xFF08, float_domain) if the computed result is NaN (only reachable if x itself was already NaN), (halt 0xFF07, float_overflow) if it is otherwise non-finite -- structurally unreachable for a result bounded to [-1, 1], kept only for the shared pack convention.
struct TanhF32 {
    x: f32,
    result: f32,
}
impl TanhF32 {
    fn run(&mut self) -> u16 {
        let neg = self.x < 0.0f32;
        let a = self.x.abs();
        let t = (-(a + a)).exp();
        let num = 1.0f32 - t;
        let den = 1.0f32 + t;
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
