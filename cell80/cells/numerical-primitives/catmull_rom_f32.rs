//! Cubic Catmull-Rom spline segment: given four control points p0, p1, p2, p3 and a fraction t in [0,1], returns the interpolated value between p1 and p2 via the standard basis-matrix formula 0.5*(2*p1 + (p2-p0)*t + (2*p0-5*p1+4*p2-p3)*t^2 + (p3-p0+3*p1-3*p2)*t^3) -- fixed at exactly this one 4-point cubic segment (the dialect has no generic spline-over-N-points capability, unlike a caller-extensible spline), distinct from lerp_f32's plain 2-point straight line since p0 and p3 shape the tangent through p1 and p2 rather than just blending two endpoints; callers combine two instances (one per axis) for a 2D curve.
//! tags: catmull-rom, catmullrom, spline, cubic-spline, interpolation, interpolate, curve, control-points, tangent, basis-matrix, four-point, f32, float, softfloat, graphics, animation, path
//! entry: CatmullRomF32::run
//! limits: t is not clamped to [0,1] by this cell (caller responsibility, mirroring lerp_f32's own contract) -- values outside [0,1] extrapolate the same cubic rather than erroring; escalates (halt 0xFF08, float_domain) if the result is NaN, or (halt 0xFF07, float_overflow) if it's non-finite
struct CatmullRomF32 {
    p0: f32,
    p1: f32,
    p2: f32,
    p3: f32,
    t: f32,
    result: f32,
}
impl CatmullRomF32 {
    fn run(&mut self) -> u16 {
        let t2 = self.t * self.t;
        let t3 = t2 * self.t;

        let c0 = 2.0f32 * self.p1;
        let c1 = (self.p2 - self.p0) * self.t;
        let c2 = (2.0f32 * self.p0 - 5.0f32 * self.p1 + 4.0f32 * self.p2 - self.p3) * t2;
        let c3 = (self.p3 - self.p0 + 3.0f32 * self.p1 - 3.0f32 * self.p2) * t3;

        let sum = c0 + c1 + c2 + c3;
        let result = 0.5f32 * sum;

        if result.is_nan() {
            halt(0xFF08u16);
        }
        if !result.is_finite() {
            halt(0xFF07u16);
        }

        self.result = result;
        1u16
    }
}
