//! The larger of two signed quantities represented as (magnitude, sign) pairs (neg 0=nonnegative, 1=negative, per smag_add), returned as its own (mag, neg) pair (ties keep a) -- the direct complement of smag_min, and unlike smag_cmp (which only returns a 0/1/2 ordering code) actually produces the winning value.
//! tags: math, signed, sign-magnitude, max, maximum, compare, order, wide, u32
//! entry: SmagMax::run
//! limits: escalates (halt 0xFF06, out_of_domain) if neg_a or neg_b is anything other than 0 or 1
struct SmagMax { mag_a: u32, neg_a: u16, mag_b: u32, neg_b: u16, mag: u32, neg: u16 }
impl SmagMax {
    fn run(&mut self) -> u16 {
        if self.neg_a > 1u16 || self.neg_b > 1u16 { halt(0xFF06u16); }
        let mut sa = self.neg_a;
        if self.mag_a == 0u32 { sa = 0u16; }
        let mut sb = self.neg_b;
        if self.mag_b == 0u32 { sb = 0u16; }
        let b_is_larger = if sa == 1u16 && sb == 0u16 {
            true
        } else if sa == 0u16 && sb == 1u16 {
            false
        } else if sa == 0u16 {
            self.mag_b > self.mag_a
        } else {
            self.mag_b < self.mag_a
        };
        let m = if b_is_larger { self.mag_b } else { self.mag_a };
        let n = if b_is_larger { sb } else { sa };
        self.mag = m;
        self.neg = n;
        1u16
    }
}
