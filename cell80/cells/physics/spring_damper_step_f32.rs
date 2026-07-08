//! One semi-implicit-Euler spring-damper step, IEEE binary32: a = -(k*x + c*v)*inv_m, then v' = v + a*dt and x' = x + v'*dt -- inverse mass as input, exactly how a Rapier-style engine stores it (and it keeps the cell division-free); non-finite state escalates instead of exploding the spring silently.
//! tags: physics, spring, damper, harmonic, oscillator, step, integrate, stiffness, f32, float, softfloat, simulation
//! entry: SpringDamperStep::run
struct SpringDamperStep {
    x: f32,
    v: f32,
    k: f32,
    c: f32,
    inv_m: f32,
    dt: f32,
    x_out: f32,
    v_out: f32,
}
impl SpringDamperStep {
    fn run(&mut self) -> u16 {
        let a = -(self.k * self.x + self.c * self.v) * self.inv_m;
        let v1 = self.v + a * self.dt;
        let x1 = self.x + v1 * self.dt;
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
