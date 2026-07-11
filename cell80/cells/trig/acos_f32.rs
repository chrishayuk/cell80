//! acos(x): arccosine of x in IEEE binary32, x constrained to [-1, 1], result in [0, pi] radians -- composed directly over the F2 owned transcendentals rather than its own polynomial, as atan2(sqrt(1-x*x), x): the square root is always >= 0, so the composed point (x, sqrt(1-x*x)) sits in the upper half-plane and fatan2's own quadrant handling walks the whole range home (x=1 -> 0, x=0 -> pi/2, x=-1 -> pi) with no separate branch needed here. A rounding guard clamps the radicand to 0.0 before the sqrt (x within [-1,1] but x*x rounding fractionally past 1.0 would otherwise sqrt a tiny negative and hand fatan2 a NaN). Distinct from asin_f32 (atan2(x, sqrt(1-x*x)) -- swapped argument order, range [-pi/2, pi/2]) and from atan2_f32 itself (a caller-supplied y/x ratio, no implicit sqrt leg).
//! tags: acos, arccosine, arc-cosine, inverse-cosine, inverse-trig, trigonometry, atan2, sqrt, radians, f32, float, softfloat
//! kernel_bank: on
//! entry: AcosF32::run
//! limits: escalates (halt 0xFF08, float_domain) if |x| > 1.0 (outside the arccosine domain), or if the composed result is NaN; escalates (halt 0xFF07, float_overflow) if the composed result is non-finite
struct AcosF32 {
    x: f32,
    result: f32,
}
impl AcosF32 {
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
        let r = y.atan2(self.x);
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
