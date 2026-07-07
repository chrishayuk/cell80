//! Compare two signed quantities represented as (magnitude, sign) pairs (neg 0=nonnegative, 1=negative, per smag_add): 0 if a < b, 1 if equal, 2 if a > b — the sign-magnitude counterpart of frac_cmp's ordering-code convention.
//! tags: math, signed, sign-magnitude, compare, cmp, order, wide, u32
//! entry: SmagCmp::run
//! limits: escalates (halt 0xFF06, out_of_domain) if neg_a or neg_b is anything other than 0 or 1
struct SmagCmp { mag_a: u32, neg_a: u16, mag_b: u32, neg_b: u16 }
impl SmagCmp {
    fn run(&mut self) -> u16 {
        if self.neg_a > 1u16 || self.neg_b > 1u16 { halt(0xFF06u16); }
        let mut sa = self.neg_a;
        if self.mag_a == 0u32 { sa = 0u16; }
        let mut sb = self.neg_b;
        if self.mag_b == 0u32 { sb = 0u16; }
        if sa == 1u16 && sb == 0u16 { return 0u16; }
        if sa == 0u16 && sb == 1u16 { return 2u16; }
        if sa == 0u16 {
            if self.mag_a < self.mag_b { return 0u16; }
            if self.mag_a > self.mag_b { return 2u16; }
            return 1u16;
        }
        if self.mag_a > self.mag_b { return 0u16; }
        if self.mag_a < self.mag_b { return 2u16; }
        1u16
    }
}
