//! asin(x): arcsine of x in IEEE binary32, x constrained to [-1, 1], result in [-pi/2, pi/2] radians -- composed directly over the F2 owned transcendentals rather than its own polynomial, as atan2(x, sqrt(1-x*x)): the square root leg is always >= 0, so the composed point (sqrt(1-x*x), x) sits in the right half-plane and fatan2's own quadrant handling carries the whole range home (x=-1 -> -pi/2, x=0 -> 0, x=1 -> pi/2) with no separate branch needed here. A rounding guard clamps the radicand to 0.0 before the sqrt (x within [-1,1] but x*x rounding fractionally past 1.0 would otherwise sqrt a tiny negative and hand fatan2 a NaN). Distinct from acos_f32 (atan2(sqrt(1-x*x), x) -- swapped argument order, range [0, pi]) and from atan2_f32 itself (a caller-supplied y/x ratio, no implicit sqrt leg).
//! tags: asin, arcsine, arc-sine, inverse-sine, inverse-trig, trigonometry, atan2, sqrt, radians, f32, float, softfloat
//! kernel_bank: on
//! entry: AsinF32::run
//! limits: escalates (halt 0xFF08, float_domain) if |x| > 1.0 (outside the arcsine domain), or if the composed result is NaN; escalates (halt 0xFF07, float_overflow) if the composed result is non-finite
struct AsinF32 {
    x: f32,
    result: f32,
}
impl AsinF32 {
    fn run(&mut self) -> u16 {
        let ax = self.x.abs();
        if ax > 1.0f32 {
            halt(0xFF08u16);
        }
        let radicand = 1.0f32 - self.x * self.x;
        let mut s = radicand;
        if s < 0.0f32 {
            s = 0.0f32;
        }
        let y = s.sqrt();
        let r = self.x.atan2(y);
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
