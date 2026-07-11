//! Cosecant csc(x) = 1/sin(x) for x in radians, composed directly over the F2 owned fsin kernel (Cody-Waite range reduction + Cephes minimax polynomial, class approximate per rustz80/src/softfloat.rs's F2 note) followed by an ordinary f32 divide -- no dedicated cosecant minimax polynomial is used, since a bare reciprocal-of-sine already inherits fsin's own bit-identical-across-targets determinism story and its shared domain wall (fsin already returns NaN for |x| > 8192.0, so CSC needs no separate range check of its own). Escalates float_domain BEFORE dividing whenever the computed sin(x) magnitude drops below a fixed near-zero epsilon, catching x close to an integer multiple of pi (a true pole, where sin(x) is zero and csc is undefined, diverging to +-infinity) as a deliberate domain decision rather than only relying on the natural Infinity/NaN a literal zero divisor would otherwise produce. Distinct from SIN itself (this cell's own reciprocal, not the sine value), from SEC (the same reciprocal move over fcos instead of fsin, with poles at odd multiples of pi/2 rather than integer multiples of pi), and from COT (cos(x)/sin(x), a ratio of two different kernels sharing the same near-zero-sine guard rather than a bare reciprocal of one).
//! tags: csc, cosecant, reciprocal-sine, reciprocal-of-sine, trig, trigonometric, sine, reciprocal, radians, pole, asymptote, f32, softfloat
//! kernel_bank: on
//! entry: CscF32::run
//! limits: escalates (halt 0xFF08, float_domain) if the computed sin(x) magnitude is below 1e-6 (a near-pole guard for x close to an integer multiple of pi, where csc diverges) BEFORE the division is attempted -- the threshold sits well above fsin's own declared absolute-error bound (<= 2^-24 over its supported domain), so genuine near-pole inputs are caught without false-triggering on ordinary rounding noise; escalates (halt 0xFF08, float_domain) if the resulting reciprocal is itself NaN, or (halt 0xFF07, float_overflow) if it is non-finite; inherits fsin's own domain wall (|x| > 8192.0 already returns NaN from the kernel, caught by the same NaN check, so no separate range guard is needed here)
struct CscF32 {
    x: f32,
    result: f32,
}
impl CscF32 {
    fn run(&mut self) -> u16 {
        let s = self.x.sin();
        let smag = s.abs();
        if smag < 0.000001f32 {
            halt(0xFF08u16);
        }
        let r = 1.0f32 / s;
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
