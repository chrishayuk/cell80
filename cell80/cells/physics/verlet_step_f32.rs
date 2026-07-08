//! One position-Verlet step under constant acceleration, IEEE binary32: x' = x + v*dt + 0.5*a*dt*dt and v' = v + a*dt -- the integrator's arithmetic exactly as a Rapier-style f32 engine computes it, correctly rounded per op; non-finite results escalate instead of corrupting the trajectory.
//! tags: physics, verlet, integrate, integrator, step, position, velocity, acceleration, f32, float, softfloat, simulation
//! entry: VerletStep::run
struct VerletStep {
    x: f32,
    v: f32,
    a: f32,
    dt: f32,
    x_out: f32,
    v_out: f32,
}
impl VerletStep {
    fn run(&mut self) -> u16 {
        let adt = self.a * self.dt;
        let x1 = self.x + self.v * self.dt + 0.5f32 * (adt * self.dt);
        let v1 = self.v + adt;
        if x1.is_nan() || v1.is_nan() {
            halt(0xFF08u16);
        }
        let xfin = x1.is_finite();
        let vfin = v1.is_finite();
        if !xfin || !vfin {
            halt(0xFF07u16);
        }
        self.x_out = x1;
        self.v_out = v1;
        1u16
    }
}
