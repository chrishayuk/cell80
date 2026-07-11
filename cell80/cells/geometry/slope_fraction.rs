//! Exact slope (y2-y1)/(x2-x1) between two integer points, returned as a sign-magnitude fraction (num_mag, num_neg) over a positive denominator (den) -- the narrower two-point sibling of linear_regression_slope's aggregated-sums fit: num and den here are plain coordinate-difference magnitudes (an excess-32768-shifted iabs_diff, the geom_distance_3d technique), never reduced to lowest terms since no multiply is involved.
//! tags: geometry, slope, line, coordinate, point, fraction, frac, signed, sign-magnitude, wide, escalate
//! entry: SlopeFraction::run
//! limits: escalates (halt 0xFF06, out_of_domain) if x1 == x2 (vertical line, undefined slope)
struct SlopeFraction { x1: i16, y1: i16, x2: i16, y2: i16, num_mag: u16, num_neg: u16, den: u16 }
impl SlopeFraction {
    fn run(&mut self) -> u16 {
        let sx1 = (self.x1 as u16).wrapping_add(32768u16);
        let sx2 = (self.x2 as u16).wrapping_add(32768u16);
        if sx1 == sx2 { halt(0xFF06u16); }
        let sy1 = (self.y1 as u16).wrapping_add(32768u16);
        let sy2 = (self.y2 as u16).wrapping_add(32768u16);

        let dx_mag = iabs_diff(sx1, sx2);
        let dx_neg = if sx2 < sx1 { 1u16 } else { 0u16 };
        let dy_mag = iabs_diff(sy1, sy2);
        let dy_neg = if sy2 < sy1 { 1u16 } else { 0u16 };

        // Normalize to a positive denominator: if dx was negative, flip the
        // numerator's sign too (a/-b == -a/b). Sign of the result is the XOR
        // of the two operand signs, forced to 0 when the numerator is 0.
        let mut num_neg = if dx_neg == dy_neg { 0u16 } else { 1u16 };
        if dy_mag == 0u16 { num_neg = 0u16; }

        self.num_mag = dy_mag;
        self.num_neg = num_neg;
        self.den = dx_mag;
        1u16
    }
}
