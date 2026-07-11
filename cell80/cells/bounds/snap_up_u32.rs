//! Round a wide u32 value x UP to the nearest multiple of step (x if step == 0 or x == 0), ceiling to grid at u32 width -- the wide sibling of snap_up (which works over u16 and can't grid-snap values beyond 65535, e.g. buffer sizes or byte offsets).
//! tags: snap, round-up, ceiling, multiple, grid, quantize, wide, u32, large, escalate
//! entry: SnapUpWide::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if the ceiling scale-back multiply (quotient * step) would exceed u32::MAX
struct SnapUpWide { x: u32, step: u32, result: u32 }
impl SnapUpWide {
    fn run(&mut self) -> u16 {
        if self.step == 0u32 || self.x == 0u32 {
            self.result = self.x;
            return 1u16;
        }
        let q = (self.x - 1u32) / self.step + 1u32;
        let r = mul_checked_u32(q, self.step);
        self.result = r;
        1u16
    }
}
