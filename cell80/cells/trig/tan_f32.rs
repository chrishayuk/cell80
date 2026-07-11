//! tan(x) = sin(x)/cos(x) for x in radians, via the F2 owned fsin/fcos kernels (Cody-Waite range reduction + Cephes minimax polynomials, class approximate) composed with an ordinary f32 divide -- no dedicated tangent minimax polynomial is used, since a bare sin-over-cos ratio already inherits both kernels' bit-identical-across-targets determinism story and their shared domain wall (fsin/fcos already return NaN for |x| > 8192.0, so TAN needs no separate range check of its own). Escalates float_domain BEFORE dividing whenever the computed cos(x) magnitude drops below a fixed near-zero epsilon, catching x close to an odd multiple of pi/2 (a true pole, where tan is undefined and diverges to +-infinity) as a deliberate domain decision rather than only relying on the natural Infinity/NaN a literal zero divisor would otherwise produce.
//! tags: tan, tangent, trig, trigonometric, sine, cosine, ratio, radians, pole, asymptote, f32, softfloat
//! kernel_bank: on
//! entry: TanF32::run
//! limits: escalates (halt 0xFF08, float_domain) if the computed cos(x) magnitude is below 1e-6 (a near-pole guard for x close to an odd multiple of pi/2, where tan diverges) BEFORE the division is attempted -- the threshold sits well above fsin/fcos's own declared absolute-error bound (<= 2^-24 over their supported domain), so genuine near-pole inputs are caught without false-triggering on ordinary rounding noise; escalates (halt 0xFF08, float_domain) if the resulting ratio is itself NaN, or (halt 0xFF07, float_overflow) if it is non-finite; inherits fsin/fcos's own domain wall (|x| > 8192.0 already returns NaN from both kernels, caught by the same NaN check, so no separate range guard is needed here)
struct TanF32 {
    x: f32,
    result: f32,
}
impl TanF32 {
    fn run(&mut self) -> u16 {
        let s = self.x.sin();
        let c = self.x.cos();
        let cmag = c.abs();
        if cmag < 0.000001f32 {
            halt(0xFF08u16);
        }
        let t = s / c;
        if t.is_nan() {
            halt(0xFF08u16);
        }
        let fin = t.is_finite();
        if !fin {
            halt(0xFF07u16);
        }
        self.result = t;
        1u16
    }
}
