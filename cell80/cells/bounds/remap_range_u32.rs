//! Linearly remaps a wide u32 value x from source range [in_lo, in_hi] into destination range [out_lo, out_hi] (x clamped into the source range first; returns out_lo if in_hi <= in_lo) -- the wide sibling of remap_range (which works over u16 and can't represent totals beyond 65535), using a checked intermediate multiply instead of remap_range's raw cast-to-u32 multiply since u32 operands can themselves overflow u32.
//! tags: remap, rescale, scale, range, interpolate, map-range, convert, transform, wide, u32, large
//! entry: RemapRangeWide::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if the intermediate multiply (clamped x - in_lo) * (out_hi - out_lo) would overflow u32
struct RemapRangeWide { x: u32, in_lo: u32, in_hi: u32, out_lo: u32, out_hi: u32, result: u32 }
impl RemapRangeWide {
    fn run(&mut self) -> u16 {
        if self.in_hi <= self.in_lo {
            self.result = self.out_lo;
        } else {
            let c = if self.x > self.in_hi { self.in_hi } else if self.x < self.in_lo { self.in_lo } else { self.x };
            let num = mul_checked_u32(c - self.in_lo, self.out_hi - self.out_lo);
            self.result = self.out_lo + num / (self.in_hi - self.in_lo);
        }
        1u16
    }
}
