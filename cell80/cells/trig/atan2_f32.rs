//! ATAN2(y, x): angle in radians of the point (x, y) from the positive x-axis, in (-pi, pi] -- a direct wrap of the shipped fatan2 kernel via Rust's own receiver-order convention `y.atan2(x)` (self is y, the argument is x, matching fatan2's own `fatan2(y, x)` signature); this is the two-argument arctangent of the ratio y/x, NOT a bare `atan(y/x)`, since it resolves the correct quadrant from the *signs* of y and x separately, including the axis cases (x == 0 or y == 0, in either sign) a single divided-down ratio can never distinguish once x's sign is gone.
//! tags: trig, atan2, arctangent, inverse-tangent, two-argument-arctangent, arctangent-of-a-ratio, angle-from-coordinates, quadrant, radians, f32, transcendental, softfloat
//! kernel_bank: on
//! entry: Atan2F32::run
//! limits: total domain -- every finite/infinite (y, x) pair, including (0, 0) and signed zeros, is defined per IEEE atan2 convention and never escalates on the inputs alone (a zero or infinite operand resolves to a specific quadrant angle, never an error); escalates (halt 0xFF08, float_domain) only if the result carries a NaN payload (only reachable if a NaN was fed in), and (halt 0xFF07, float_overflow) if the result is otherwise non-finite -- not expected anywhere in this kernel's domain (its output is always bounded to (-pi, pi]), kept only as the shared defensive check every f32 cell in this library runs before trusting a result.
struct Atan2F32 {
    y: f32,
    x: f32,
    angle: f32,
}
impl Atan2F32 {
    fn run(&mut self) -> u16 {
        let a = self.y.atan2(self.x);
        if a.is_nan() {
            halt(0xFF08u16);
        }
        let fin = a.is_finite();
        if !fin {
            halt(0xFF07u16);
        }
        self.angle = a;
        1u16
    }
}
