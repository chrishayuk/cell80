//! Linear interpolation a + t*(b - a) in IEEE binary32 — the owned softfloat kernels, correctly rounded per op and bit-identical to rustc f32; t is a plain f32 (not clamped), so t=0 gives a and t=1 gives a + (b - a).
//! tags: f32, float, softfloat, lerp, interpolate, blend, mix, tween, ease
//! entry: LerpF32::run
struct LerpF32 {
    a: f32,
    b: f32,
    t: f32,
    out: f32,
}
impl LerpF32 {
    fn run(&mut self) -> u16 {
        self.out = self.a + self.t * (self.b - self.a);
        1u16
    }
}
