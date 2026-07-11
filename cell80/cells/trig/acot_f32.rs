//! ACOT(x): arccotangent, the angle whose cotangent is x, returned in the continuous open interval (0, pi) -- Excel's own documented range and convention, distinct from the discontinuous "atan(1/x) with only x=0 patched" shape some libraries call arccotangent. Composed directly as atan2(1, x) through the single fatan2 kernel (no reciprocal, no series of its own): a point (x, 1) always sits in the upper half-plane (y = 1 > 0), so its atan2 angle is forced into (0, pi) by construction, and by definition that angle's cotangent (its x-coordinate over its y-coordinate, x/1 = x) is exactly x -- so atan2(1, x) *is* arccotangent, not merely an approximation of it. This also gives acot(0) = pi/2 for free as an ordinary interior point of that construction (fatan2's own xmag==0 branch resolves atan2(1, 0) to pi/2 directly), matching Excel's explicit acot(0) = pi/2 special case with no domain error and no reciprocal 1/0 to guard against, unlike a naive atan(1/x) formula which would divide by zero right at x=0 and need its own patch. The identity atan2(1, x) = pi/2 - atan(x) holds for every real x (verified algebraically via the complementary-angle relation, not assumed), so this is also exactly Excel's stated ACOT-ATAN relationship, just built without ever computing atan(x) or a reciprocal at all. Distinct from ATAN (this pack's atan2(x, 1) composition -- note the arguments are in the OPPOSITE order from ACOT's atan2(1, x), and ATAN's principal range is (-pi/2, pi/2), not (0, pi)), from ATAN2 (the general two-argument arctangent of an arbitrary ratio y/x with both signs independently significant, not this cell's fixed y=1), and from ACOTH (inverse HYPERBOLIC cotangent, 0.5*ln((x+1)/(x-1)) over fln, a circular-vs-hyperbolic distinction the acoth_f32.rs header itself already draws pointing forward to this cell).
//! tags: acot, arccotangent, arc-cotangent, inverse-cotangent, inverse-trig, trig, trigonometric, cotangent, atan2, fatan2, angle-from-cotangent, radians, f32, float, softfloat, transcendental
//! kernel_bank: on
//! entry: AcotF32::run
//! limits: no domain restriction -- every finite x, plus x = 0 (the explicit Excel special case, resolved to pi/2, not a domain error) and +/-infinity (resolving to the 0 and pi asymptotes), has a defined result in (0, pi); escalates (halt 0xFF08, float_domain) only if the computed result is NaN (reachable only when x itself was already NaN), (halt 0xFF07, float_overflow) only if it is non-finite (structurally unreachable for a result bounded to (0, pi), kept only for the shared pack convention every f32 cell here follows).
struct AcotF32 {
    x: f32,
    result: f32,
}
impl AcotF32 {
    fn run(&mut self) -> u16 {
        let r = 1.0f32.atan2(self.x);
        if r.is_nan() {
            halt(0xFF08u16);
        }
        let fin = r.is_finite();
        if !fin {
            halt(0xFF07u16);
        }
        self.result = r;
        1u16
    }
}
