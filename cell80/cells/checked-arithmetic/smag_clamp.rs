//! Clamp a signed value (mag_x, neg_x) into an inclusive signed range [lo, hi] (each given as its own mag/neg pair), using the same sign-then-magnitude comparison smag_cmp/smag_max already implement -- the sign-magnitude sibling of clamp_u32 (wide but unsigned only) and distinct from apply_delta_clamped_u32 (which clamps an unsigned pool value after a signed delta and cannot represent a negative lower bound).
//! tags: math, signed, sign-magnitude, clamp, bound, bounds, limit, restrict, constrain, range, wide, u32
//! entry: SmagClamp::run
//! limits: escalates (halt 0xFF06, out_of_domain) if any of neg_x, neg_lo, neg_hi is anything other than 0 or 1
struct SmagClamp { mag_x: u32, neg_x: u16, mag_lo: u32, neg_lo: u16, mag_hi: u32, neg_hi: u16, mag: u32, neg: u16 }
impl SmagClamp {
    fn run(&mut self) -> u16 {
        if self.neg_x > 1u16 || self.neg_lo > 1u16 || self.neg_hi > 1u16 { halt(0xFF06u16); }
        let mut sx = self.neg_x;
        if self.mag_x == 0u32 { sx = 0u16; }
        let mut sl = self.neg_lo;
        if self.mag_lo == 0u32 { sl = 0u16; }
        let mut sh = self.neg_hi;
        if self.mag_hi == 0u32 { sh = 0u16; }

        let x_lt_lo = if sx == 1u16 && sl == 0u16 {
            true
        } else if sx == 0u16 && sl == 1u16 {
            false
        } else if sx == 0u16 {
            self.mag_x < self.mag_lo
        } else {
            self.mag_x > self.mag_lo
        };

        let x_gt_hi = if sx == 1u16 && sh == 0u16 {
            false
        } else if sx == 0u16 && sh == 1u16 {
            true
        } else if sx == 0u16 {
            self.mag_x > self.mag_hi
        } else {
            self.mag_x < self.mag_hi
        };

        let out_mag = if x_lt_lo { self.mag_lo } else if x_gt_hi { self.mag_hi } else { self.mag_x };
        let out_neg = if x_lt_lo { sl } else if x_gt_hi { sh } else { sx };
        self.mag = out_mag;
        self.neg = out_neg;
        1u16
    }
}
