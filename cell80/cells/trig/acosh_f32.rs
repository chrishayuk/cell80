//! Inverse hyperbolic cosine ACOSH(x) = ln(x + sqrt(x*x - 1)), defined only for x >= 1 (cosh(y) >= 1 for every real y, so no smaller x has a real preimage) -- composed directly from the two kernels already on the f32 method surface: `.sqrt()` (the correctly-rounded, exact fsqrt) feeding `.ln()` (the <=1 ulp fln), no series or iteration of its own. The first cell in a new trig pack riding the F2 owned transcendentals (fsin/fcos/fatan2/fexp/fln/fpow, rustz80/src/softfloat.rs). Distinct from ASINH (x + sqrt(x*x + 1) under the same ln, defined over all reals, never escalates on x itself) and from ATANH (ln((1+x)/(1-x))/2, a ratio-of-sums over the open interval (-1,1), not a sum-under-a-square-root); also distinct from COSH itself (the forward hyperbolic cosine (e^x+e^-x)/2, which never escalates on domain since every real x is valid) -- this cell runs the inverse direction, solving for y given cosh(y) = x.
//! tags: trig, hyperbolic, inverse-hyperbolic-cosine, hyperbolic-cosine-inverse, arcosh, acosh, inverse-cosh, ln, sqrt, transcendental, f32, float, softfloat, math-trig
//! kernel_bank: on
//! entry: AcoshF32::run
//! accuracy: <= 2 ulp (fsqrt is correctly-rounded/exact per rustz80's F0 harness; the single fln call carries <= 1 ulp measured against offline-MPFR golden tables, rustz80's F2 harness pins the kernel -- see rustz80/tests/diff/f32_trans.rs)
//! limits: defined for x >= 1 only -- escalates (halt 0xFF08, float_domain) if x < 1 (the same code a NaN result would get, since a sub-1 x would otherwise drive sqrt's argument negative and produce exactly that NaN if this check weren't taken first); escalates (halt 0xFF08, float_domain) if the sqrt or ln step itself produces NaN, or (halt 0xFF07, float_overflow) if either produces a non-finite result -- both reachable only from an already non-finite x, since a finite x >= 1 always has a finite, non-NaN acosh
struct AcoshF32 {
    x: f32,
    result: f32,
}
impl AcoshF32 {
    fn run(&mut self) -> u16 {
        if self.x < 1.0f32 {
            halt(0xFF08u16);
        }
        let inner = self.x * self.x - 1.0f32;
        let root = inner.sqrt();
        if root.is_nan() {
            halt(0xFF08u16);
        }
        let root_fin = root.is_finite();
        if !root_fin {
            halt(0xFF07u16);
        }
        let sum = self.x + root;
        let result = sum.ln();
        if result.is_nan() {
            halt(0xFF08u16);
        }
        let fin = result.is_finite();
        if !fin {
            halt(0xFF07u16);
        }
        self.result = result;
        1u16
    }
}
