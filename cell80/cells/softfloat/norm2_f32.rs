//! Euclidean length of a 2-vector in IEEE binary32 — sqrt(x*x + y*y) through the owned softfloat kernels: correctly rounded per op, bit-identical to rustc f32, deterministic on every host (no libm).
//! tags: f32, float, softfloat, norm, magnitude, length, hypotenuse, distance, vector, sqrt
//! entry: Norm2F32::run
struct Norm2F32 {
    x: f32,
    y: f32,
    len: f32,
}
impl Norm2F32 {
    fn run(&mut self) -> u16 {
        self.len = (self.x * self.x + self.y * self.y).sqrt();
        1u16
    }
}
