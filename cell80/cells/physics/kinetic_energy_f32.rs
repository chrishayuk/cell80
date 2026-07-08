//! Kinetic energy 0.5*m*v*v in IEEE binary32 through the owned softfloat kernels -- correctly rounded per op, bit-identical to rustc f32; escalates float_overflow/float_domain instead of reporting a non-finite energy.
//! tags: physics, kinetic, energy, mass, velocity, f32, float, softfloat, mechanics
//! entry: KineticEnergy::run
struct KineticEnergy {
    m: f32,
    v: f32,
    e: f32,
}
impl KineticEnergy {
    fn run(&mut self) -> u16 {
        let e = 0.5f32 * self.m * (self.v * self.v);
        if e.is_nan() {
            halt(0xFF08u16);
        }
        let fin = e.is_finite();
        if !fin {
            halt(0xFF07u16);
        }
        self.e = e;
        1u16
    }
}
