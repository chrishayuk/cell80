//! Cosine of the angle opposite side c in a triangle with integer sides (a, b, c), via the law of cosines rearranged to an exact fraction: cos C = (a² + b² − c²) / (2ab) — no square root, no trig, just integer arithmetic. Returned as a sign-magnitude fraction (mag_num, neg_num, den) since the numerator is negative whenever angle C is obtuse; reduced to lowest terms via the shared gcd_u32 kernel.
//! tags: geometry, triangle, angle, cosine, law-of-cosines, fraction, frac, signed, sign-magnitude, wide, u32, checked, escalate, aime
//! entry: CosFracFromSides::run
//! limits: escalates (halt 0xFF06, out_of_domain) if a, b, c do not form a valid (non-degenerate) triangle; escalates (halt 0xFF05, needs_wider_math) if any squared term or 2ab overflows u32
struct CosFracFromSides { a: u16, b: u16, c: u16, mag_num: u32, neg_num: u16, den: u32 }
impl CosFracFromSides {
    fn run(&mut self) -> u16 {
        let aw = self.a as u32;
        let bw = self.b as u32;
        let cw = self.c as u32;
        if aw + bw <= cw || bw + cw <= aw || aw + cw <= bw { halt(0xFF06u16); }
        let a2 = mul_checked_u32(aw, aw);
        let b2 = mul_checked_u32(bw, bw);
        let c2 = mul_checked_u32(cw, cw);
        let ab_sum = add_checked_u32(a2, b2);
        let mag = if ab_sum >= c2 { ab_sum - c2 } else { c2 - ab_sum };
        let neg = if ab_sum >= c2 { 0u16 } else { 1u16 };
        let den_raw = mul_checked_u32(2u32, mul_checked_u32(aw, bw));
        if mag == 0u32 {
            self.mag_num = 0u32;
            self.neg_num = 0u16;
            self.den = 1u32;
            return 1u16;
        }
        let g = gcd_u32(mag, den_raw);
        self.mag_num = mag / g;
        self.neg_num = neg;
        self.den = den_raw / g;
        1u16
    }
}
