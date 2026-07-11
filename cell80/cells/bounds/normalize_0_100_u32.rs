//! Rescale a wide u32 value x within [lo, hi] to a 0..100 percentage (clamped first; 0 if hi <= lo) -- the wide sibling of normalize_0_100 (which works over u16 and can't represent totals beyond 65535, e.g. mapping a wide money total into a 0..100 percentage).
//! tags: normalize, rescale, scale, percent, map-range, proportion, wide, u32, large
//! entry: NormalizeWide::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if the intermediate multiply (clamped x - lo) * 100 would overflow u32
struct NormalizeWide { x: u32, lo: u32, hi: u32, result: u32 }
impl NormalizeWide {
    fn run(&mut self) -> u16 {
        if self.hi > self.lo {
            let c = if self.x > self.hi { self.hi } else if self.x < self.lo { self.lo } else { self.x };
            let num = mul_checked_u32(c - self.lo, 100u32);
            self.result = num / (self.hi - self.lo);
        } else {
            self.result = 0u32;
        }
        1u16
    }
}
