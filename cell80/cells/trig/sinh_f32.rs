//! Hyperbolic sine sinh(x) = (e^x - e^-x) / 2, composed directly from two independent calls into the shared F2 fexp kernel (e^x via x.exp(), e^-x via (-x).exp() -- the same kernel evaluated at the negated input, not a separate reciprocal/fdiv path) and one exact halving (a power-of-two multiply, no rounding of its own) -- unlike ATANH (this pack's other hyperbolic cell, built over one fln call and defined only on the open interval (-1, 1)) sinh has no domain restriction and grows unbounded and monotonically with x, and unlike TAN/SIN/COS (the circular side, reduced against pi through Cody-Waite and bounded/periodic by construction) sinh never wraps: there is no domain wall here beyond f32's own overflow threshold, since e^|x| itself is what eventually overflows, exactly matching sinh's real unbounded growth.
//! tags: hyperbolic, hyperbolic-sine, sinh, exponential, exp, fexp, unbounded, monotonic, odd-function, f32, float, softfloat, trig
//! kernel_bank: on
//! entry: SinhF32::run
//! accuracy: <= 2 ulp away from x == 0.0 (two independent fexp evaluations, each <= 1 ulp, combined through one exact-rounded subtract and one exact halving); known limit near x == 0.0: e^x and e^-x both round to ~1.0 there, so their difference suffers catastrophic cancellation against each fexp call's own ~1 ulp absolute error -- the same class of honestly-documented gap excel_nper.rs/excel_pduration.rs already flag for ln(1+rate) at small rate, not hidden.
//! limits: sinh has no restricted domain -- every finite x has a defined result; escalates (halt 0xFF08, float_domain) if the computed result is NaN (only reachable if x itself was already NaN), (halt 0xFF07, float_overflow) if it is non-finite -- reachable once |x| grows enough that e^|x| itself overflows f32 (a bit past x ~= +/-88.7), matching sinh's own real unbounded growth rather than any artificial cutoff.
struct SinhF32 {
    x: f32,
    result: f32,
}
impl SinhF32 {
    fn run(&mut self) -> u16 {
        let ep = self.x.exp();
        let en = (-self.x).exp();
        let diff = ep - en;
        let r = diff * 0.5f32;
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
