//! Wide u32 sibling of value_at_percent: lo + (hi-lo)*pct/100 at u32 width (pct clamped to 100), escalating on intermediate multiply overflow instead of wrapping (returns lo if hi <= lo).
//! tags: percent, value, range, interpolate, denormalize, inverse, unscale, lerp, map-range, unnormalize, wide, u32, large
//! entry: ValueAtPercentWide::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if the intermediate multiply (hi-lo)*pct would overflow u32
struct ValueAtPercentWide { lo: u32, hi: u32, pct: u32, result: u32 }
impl ValueAtPercentWide {
    fn run(&mut self) -> u16 {
        if self.hi <= self.lo {
            self.result = self.lo;
        } else {
            let p = if self.pct > 100u32 { 100u32 } else { self.pct };
            let span = self.hi - self.lo;
            let num = mul_checked_u32(span, p);
            self.result = self.lo + num / 100u32;
        }
        1u16
    }
}
