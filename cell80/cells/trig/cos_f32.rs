//! cos(x): cosine of x in radians, IEEE binary32 -- a direct wrap of the F2 owned fcos kernel (Cody-Waite range reduction plus a Cephes single-precision minimax polynomial, class approximate per rustz80/src/softfloat.rs's F2 note), landing in this fresh trig pack alongside atan2_f32/atan_f32/acos_f32/atanh_f32/tan_f32 as the plain wrap over one shipped transcendental kernel with no composition of its own. Distinct from cos_frac_from_sides (a wholly separate, exact rational cosine of a triangle's included angle from its three integer side lengths via the law of cosines -- no radians, no float, no kernel call at all) and from TAN (which composes this same pack's fsin AND fcos together as a ratio) -- this cell calls fcos alone, with no division and no second kernel.
//! tags: trig, trigonometry, cosine, cos, radians, angle, periodic, circular, waveform, f32, float, softfloat, transcendental
//! kernel_bank: on
//! entry: CosF32::run
//! limits: fcos's own domain wall: |x| > 8192.0 radians returns the kernel's canonical quiet NaN rather than a numerically-unreliable range-reduced result (past that magnitude a binary32 mantissa can no longer resolve which multiple of pi/2 x lands near) -- escalates (halt 0xFF08, float_domain) on that NaN or on any NaN/Inf input, (halt 0xFF07, float_overflow) on a non-finite result (cos is mathematically bounded to [-1,1], so this is the same belt-and-braces check every other f32 cell in this pack runs, not an expected path)
struct CosF32 {
    x: f32,
    result: f32,
}
impl CosF32 {
    fn run(&mut self) -> u16 {
        let r = self.x.cos();
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
