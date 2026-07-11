//! sin(x): sine of x in radians, IEEE binary32 -- a direct wrap of the F2 owned fsin kernel (Cody-Waite range reduction plus a Cephes single-precision minimax polynomial, class approximate per rustz80/src/softfloat.rs's F2 note), landing in this fresh trig pack as the plain wrap over one shipped transcendental kernel with no composition of its own -- the exact counterpart of cos_f32 (same kernel family, same domain wall, sine instead of cosine) and distinct from TAN (which composes this pack's fsin AND fcos together as a ratio, with its own near-pole guard) and from ATAN2 (the two-argument arctangent of a ratio, an inverse operation entirely, not a forward trig call at all). This cell calls fsin alone, with no division and no second kernel.
//! tags: trig, trigonometry, sine, sin, radians, angle, periodic, circular, waveform, f32, float, softfloat, transcendental
//! kernel_bank: on
//! entry: SinF32::run
//! limits: fsin's own domain wall: |x| > 8192.0 radians returns the kernel's canonical quiet NaN rather than a numerically-unreliable range-reduced result (past that magnitude a binary32 mantissa can no longer resolve which multiple of pi/2 x lands near) -- escalates (halt 0xFF08, float_domain) on that NaN or on any NaN/Inf input, (halt 0xFF07, float_overflow) on a non-finite result (sin is mathematically bounded to [-1,1], so this is the same belt-and-braces check every other f32 cell in this pack runs, not an expected path)
struct SinF32 {
    x: f32,
    result: f32,
}
impl SinF32 {
    fn run(&mut self) -> u16 {
        let r = self.x.sin();
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
