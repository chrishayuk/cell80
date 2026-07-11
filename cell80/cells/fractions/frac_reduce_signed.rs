//! Reduce a signed sign-magnitude fraction (mag/den, sign carried separately as neg) to lowest terms via the shared gcd_u32 kernel, canonicalizing neg to 0 whenever the reduced magnitude is 0 -- the signed counterpart of frac_reduce for the (magnitude, neg, denominator) shape linear_regression_slope/intercept, cos_frac_from_sides, and slope_fraction all return unreduced.
//! tags: fraction, frac, reduce, lowest-terms, gcd, signed, sign-magnitude, wide, u32, checked, escalate
//! entry: FracReduceSigned::run
//! limits: escalates (halt 0xFF06, out_of_domain) if den == 0
struct FracReduceSigned { mag: u32, neg: u16, den: u32, out_mag: u32, out_neg: u16, out_den: u32 }
impl FracReduceSigned {
    fn run(&mut self) -> u16 {
        if self.den == 0u32 { halt(0xFF06u16); }
        let g = gcd_u32(self.mag, self.den);
        let out_mag = self.mag / g;
        let out_neg = if out_mag == 0u32 { 0u16 } else { self.neg };
        self.out_mag = out_mag;
        self.out_neg = out_neg;
        self.out_den = self.den / g;
        1u16
    }
}
