//! sec(x) = 1/cos(x) for x in radians, composed directly over the F2 owned fcos kernel (Cody-Waite range reduction + Cephes minimax polynomial, class approximate) followed by an ordinary f32 divide -- no dedicated secant minimax polynomial is used, since a bare reciprocal-of-cosine already inherits fcos's own bit-identical-across-targets determinism story and its shared domain wall (fcos already returns NaN for |x| > 8192.0, so SEC needs no separate range check of its own). Escalates float_domain BEFORE dividing whenever the computed cos(x) magnitude drops below a fixed near-zero epsilon, catching x close to an odd multiple of pi/2 (a true pole, where cos(x) is zero and sec is undefined, diverging to +-infinity) as a deliberate domain decision rather than only relying on the natural Infinity/NaN a literal zero divisor would otherwise produce. Distinct from COS itself (this cell's own reciprocal, not the cosine value) and from TAN (sin(x)/cos(x), a ratio of two different kernels sharing the same near-pole guard rather than a bare reciprocal of one).
//! tags: sec, secant, reciprocal-cosine, reciprocal-of-cosine, trig, trigonometric, cosine, reciprocal, radians, pole, asymptote, f32, softfloat
//! kernel_bank: on
//! entry: SecF32::run
//! limits: escalates (halt 0xFF08, float_domain) if the computed cos(x) magnitude is below 1e-6 (a near-pole guard for x close to an odd multiple of pi/2, where sec diverges) BEFORE the division is attempted -- the threshold sits well above fcos's own declared absolute-error bound (<= 2^-24 over its supported domain), so genuine near-pole inputs are caught without false-triggering on ordinary rounding noise; escalates (halt 0xFF08, float_domain) if the resulting reciprocal is itself NaN, or (halt 0xFF07, float_overflow) if it is non-finite; inherits fcos's own domain wall (|x| > 8192.0 already returns NaN from the kernel, caught by the same NaN check, so no separate range guard is needed here)
struct SecF32 {
    x: f32,
    result: f32,
}
impl SecF32 {
    fn run(&mut self) -> u16 {
        let c = self.x.cos();
        let cmag = c.abs();
        if cmag < 0.000001f32 {
            halt(0xFF08u16);
        }
        let r = 1.0f32 / c;
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
