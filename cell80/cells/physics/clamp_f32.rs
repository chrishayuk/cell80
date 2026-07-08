//! Clamp an f32 into [lo, hi] via max-then-min (x.max(lo).min(hi)) -- the branch-free form; NaN x resolves to lo (min/max treat NaN as missing data, the documented divergence from f32::clamp's NaN-propagating semantics), so the output is always a real bound.
//! tags: physics, clamp, bound, limit, saturate, range, f32, float, softfloat
//! entry: ClampF32::run
struct ClampF32 {
    x: f32,
    lo: f32,
    hi: f32,
    out: f32,
}
impl ClampF32 {
    fn run(&mut self) -> u16 {
        self.out = self.x.max(self.lo).min(self.hi);
        1u16
    }
}
