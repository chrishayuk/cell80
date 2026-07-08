//! 1D collision impulse with restitution, IEEE binary32: j = -(1+e)*(v1 - v2) / (inv_m1 + inv_m2) -- inverse masses as inputs (the Rapier convention; a static body is inv_m = 0, and two static bodies make the denominator 0 -> j = +/-Inf -> float_overflow escalation, never a silent explosion).
//! tags: physics, impulse, collision, restitution, contact, momentum, f32, float, softfloat, mechanics
//! entry: Impulse1d::run
struct Impulse1d {
    e: f32,
    v1: f32,
    v2: f32,
    inv_m1: f32,
    inv_m2: f32,
    j: f32,
}
impl Impulse1d {
    fn run(&mut self) -> u16 {
        let vr = self.v1 - self.v2;
        let j = -((1.0f32 + self.e) * vr) / (self.inv_m1 + self.inv_m2);
        if j.is_nan() {
            halt(0xFF08u16);
        }
        let fin = j.is_finite();
        if !fin {
            halt(0xFF07u16);
        }
        self.j = j;
        1u16
    }
}
