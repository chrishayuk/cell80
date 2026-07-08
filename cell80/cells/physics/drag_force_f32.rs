//! Quadratic drag force k*v*|v| in IEEE binary32 -- signed (opposes the sign of v when the caller negates), correctly rounded per op through the owned softfloat kernels; a non-finite force escalates instead of flowing onward.
//! tags: physics, drag, force, quadratic, velocity, f32, float, softfloat, aerodynamics
//! entry: DragForce::run
struct DragForce {
    k: f32,
    v: f32,
    f: f32,
}
impl DragForce {
    fn run(&mut self) -> u16 {
        let f = self.k * (self.v * self.v.abs());
        if f.is_nan() {
            halt(0xFF08u16);
        }
        let fin = f.is_finite();
        if !fin {
            halt(0xFF07u16);
        }
        self.f = f;
        1u16
    }
}
