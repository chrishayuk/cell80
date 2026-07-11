//! Hyperbolic cosecant csch(x) = 1/sinh(x) = 2/(e^x - e^-x), x != 0.0, composed directly over two independent calls into the shared F2 fexp kernel (e^x via x.exp(), e^-x via (-x).exp() -- the same kernel evaluated at the negated input, not a separate reciprocal/fdiv path bolted onto SINH itself) followed by one exact halving and one ordinary f32 divide -- no dedicated cosecant-hyperbolic minimax polynomial is used, since sinh's own closed form already gives an exact reciprocal path. csch has a true pole at x == 0.0 (sinh(0.0) == 0.0 exactly, the only zero sinh ever takes since it is strictly monotonic), so this cell escalates float_domain whenever |x| falls below a fixed near-zero epsilon BEFORE either exponential is even computed, catching the pole as a deliberate domain decision rather than only relying on the natural Infinity a literal zero divisor would otherwise produce. Distinct from SINH (this pack's own reciprocal target, not the hyperbolic-sine value itself), from SEC (the circular reciprocal of fcos, periodic with poles at every odd multiple of pi/2 rather than only at zero), and from COSH (the reciprocal of hyperbolic cosine, which this pack does not yet ship and which would have no pole at all since cosh is never zero).
//! tags: csch, cosecant-hyperbolic, hyperbolic-cosecant, reciprocal-hyperbolic-sine, reciprocal-of-sinh, hyperbolic, exponential, exp, fexp, pole, asymptote, odd-function, f32, float, softfloat, trig
//! kernel_bank: on
//! entry: CschF32::run
//! accuracy: <= ~3 ulp away from the pole (two independent fexp evaluations, each <= 1 ulp, combined through one exact-rounded subtract, one exact halving, and one f32 divide); known limit just outside the escalation guard: as x approaches 0.0 the denominator e^x - e^-x suffers the same catastrophic cancellation SINH's own doc comment already flags, so csch's relative accuracy is weakest closest to the pole even though the upfront |x| guard keeps the pole itself from ever reaching the divide.
//! limits: escalates (halt 0xFF08, float_domain) if |x| < 1e-6 -- the pole at x == 0.0 (sinh(0.0) == 0.0 exactly, csch undefined/diverging to +/-infinity there) is caught upfront rather than only relying on the natural Infinity a literal zero divisor would otherwise produce; escalates (halt 0xFF08, float_domain) if the computed reciprocal is itself NaN, or (halt 0xFF07, float_overflow) if it is non-finite -- reachable once |x| grows enough that e^|x| itself overflows f32 (a bit past x ~= +/-88.7, fexp's own overflow threshold), matching csch's own real unbounded-then-vanishing shape away from the pole.
struct CschF32 {
    x: f32,
    result: f32,
}
impl CschF32 {
    fn run(&mut self) -> u16 {
        let ax = self.x.abs();
        if ax < 0.000001f32 {
            halt(0xFF08u16);
        }
        let ep = self.x.exp();
        let en = (-self.x).exp();
        let diff = ep - en;
        let sh = diff * 0.5f32;
        let r = 1.0f32 / sh;
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
