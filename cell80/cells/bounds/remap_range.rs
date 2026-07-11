//! General linear remap of x from [in_lo, in_hi] to [out_lo, out_hi]: clamp x into the input range, then out_lo + (x-in_lo)*(out_hi-out_lo)/(in_hi-in_lo) (returns out_lo if in_hi <= in_lo) -- normalize_0_100 (output fixed to [0,100]) and value_at_percent (input fixed to [0,100]) are both special cases of this fully general two-arbitrary-range map.
//! tags: remap, range, rescale, scale, map-range, interpolate, linear, proportion, lerp, unnormalize, two-range
//! entry: RemapRange::run
struct RemapRange { x: u16, in_lo: u16, in_hi: u16, out_lo: u16, out_hi: u16, result: u16 }
impl RemapRange {
    fn run(&mut self) -> u16 {
        if self.in_hi <= self.in_lo {
            self.result = self.out_lo;
        } else {
            let c = if self.x > self.in_hi { self.in_hi } else if self.x < self.in_lo { self.in_lo } else { self.x };
            let num = (c - self.in_lo) as u32 * (self.out_hi - self.out_lo) as u32;
            let scaled = num / (self.in_hi - self.in_lo) as u32;
            self.result = self.out_lo + scaled as u16;
        }
        1u16
    }
}
