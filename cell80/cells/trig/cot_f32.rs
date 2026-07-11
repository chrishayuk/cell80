//! Cotangent cot(x) = cos(x)/sin(x) = 1/tan(x) in radians -- composed directly from the F2 fcos/fsin kernels (two owned-transcendental calls, no separate cotangent kernel of its own) rather than as tan_f32's own reciprocal (1.0/tan(x), which would chain a second division on top of tan's own sin/cos divide): computing cos(x) and sin(x) once each here and dividing straight through avoids that double division and shares no intermediate with tan_f32, even though the two functions are algebraic reciprocals of one another. Distinct from tan_f32 itself (sin(x)/cos(x), escalating when cos(x) is ~0 instead) since each escalates on its OWN denominator collapsing to zero, never the other's, and distinct from acot_f32 (the inverse function -- given a cotangent value, recover the angle -- not this forward ratio) despite the shared name root.
//! tags: cot, cotangent, trig, trigonometry, reciprocal-tangent, tangent, cosine, sine, fsin, fcos, radians, f32, float, softfloat
//! kernel_bank: on
//! entry: CotF32::run
//! limits: escalates (halt 0xFF08, float_domain) if |sin(x)| < 1e-6 (x sits within about a microradian of a multiple of pi, where cot diverges toward +/-infinity) -- checked BEFORE the divide rather than caught only as a non-finite result afterward; escalates (halt 0xFF08, float_domain) if fsin/fcos itself returns NaN, which happens for a NaN/infinite x or for any |x| > 8192.0 rad (the F2 fsin/fcos kernels' own documented valid-input ceiling in rustz80/src/softfloat.rs -- beyond it they return a quiet NaN rather than a reduced angle); escalates (halt 0xFF07, float_overflow) if the computed cotangent is otherwise non-finite.
struct CotF32 {
    x: f32,
    result: f32,
}
impl CotF32 {
    fn run(&mut self) -> u16 {
        let s = self.x.sin();
        let c = self.x.cos();
        if s.is_nan() || c.is_nan() {
            halt(0xFF08u16);
        }
        let s_fin = s.is_finite();
        let c_fin = c.is_finite();
        if !s_fin || !c_fin {
            halt(0xFF07u16);
        }
        let smag = s.abs();
        if smag < 0.000001f32 {
            halt(0xFF08u16);
        }
        let cot = c / s;
        if cot.is_nan() {
            halt(0xFF08u16);
        }
        let cot_fin = cot.is_finite();
        if !cot_fin {
            halt(0xFF07u16);
        }
        self.result = cot;
        1u16
    }
}
