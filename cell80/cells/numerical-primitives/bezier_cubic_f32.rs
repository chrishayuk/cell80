//! Point on a cubic Bezier curve at parameter t via De Casteljau's algorithm (three rounds of repeated lerp -- lerp_f32's own a + t*(b-a) technique, inlined: p0..p3 first fold to ab/bc/cd, those fold to abc/bcd, then to the final point), algebraically identical to the direct Bernstein form (1-t)^3*p0 + 3*(1-t)^2*t*p1 + 3*(1-t)*t^2*p2 + t^3*p3 -- unlike catmull_rom_f32's Hermite-basis spline, where all four control points sit ON the curve, here only p0/p3 lie on the curve while p1/p2 merely pull its tangent/shape without ever being visited.
//! tags: bezier, cubic-bezier, decasteljau, de-casteljau, bernstein, curve, spline, control-points, interpolate, path, animation, easing, tween, f32, float, softfloat
//! entry: BezierCubicF32::run
struct BezierCubicF32 {
    p0: f32,
    p1: f32,
    p2: f32,
    p3: f32,
    t: f32,
    out: f32,
}
impl BezierCubicF32 {
    fn run(&mut self) -> u16 {
        // First round: three lerps between adjacent control points.
        let ab = self.p0 + self.t * (self.p1 - self.p0);
        let bc = self.p1 + self.t * (self.p2 - self.p1);
        let cd = self.p2 + self.t * (self.p3 - self.p2);
        // Second round: lerp those down to two points.
        let abc = ab + self.t * (bc - ab);
        let bcd = bc + self.t * (cd - bc);
        // Third round: the final point on the curve.
        let out = abc + self.t * (bcd - abc);
        if out.is_nan() {
            halt(0xFF08u16);
        }
        let fin = out.is_finite();
        if !fin {
            halt(0xFF07u16);
        }
        self.out = out;
        1u16
    }
}
