//! ATAN(x): arctangent of x alone, in radians, returned in the open interval (-pi/2, pi/2) -- composed directly as atan2(x, 1.0) through the single fatan2 kernel (no series expansion or range reduction of its own): unlike ATAN2 (the two-argument arctangent of a ratio y/x that recovers the correct quadrant from the independent signs of y and x, and can return any angle across the full (-pi, pi] range) this cell takes one bare already-divided value and is pinned to Excel's own ATAN convention (the principal value alone, x's own sign giving the output's sign, x=0 giving exactly 0); also distinct from ATANH (inverse hyperbolic tangent, ln((1+x)/(1-x))/2, defined only on the open interval (-1, 1) and built over fln rather than fatan2) despite the shared "atan" name root.
//! tags: atan, arctangent, arctan, inverse-tangent, inverse-trig, trig, angle, radians, atan2, fatan2, f32, float, softfloat
//! kernel_bank: on
//! entry: AtanF32::run
//! limits: no domain restriction -- every real x (including +/-infinity, which resolve to the +/-pi/2 asymptote) has a defined result; escalates (halt 0xFF08, float_domain) only if the computed result is NaN (reachable only when x itself was already NaN), (halt 0xFF07, float_overflow) only if it is non-finite (structurally unreachable for a result bounded to (-pi/2, pi/2), kept only for the shared pack convention)
struct AtanF32 {
    x: f32,
    result: f32,
}
impl AtanF32 {
    fn run(&mut self) -> u16 {
        let r = self.x.atan2(1.0f32);
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
