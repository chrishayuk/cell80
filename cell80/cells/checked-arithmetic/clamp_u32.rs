//! Clamp a wide u32 value to the inclusive range [lo, hi] — the wide sibling of clamp (which works over u16).
//! tags: math, clamp, bound, bounds, limit, restrict, constrain, floor, ceiling, range, wide, u32, large
//! entry: ClampWide::run
struct ClampWide { x: u32, lo: u32, hi: u32, result: u32 }
impl ClampWide {
    fn run(&mut self) -> u16 {
        let v = if self.x > self.hi { self.hi } else if self.x < self.lo { self.lo } else { self.x };
        self.result = v;
        1u16
    }
}
