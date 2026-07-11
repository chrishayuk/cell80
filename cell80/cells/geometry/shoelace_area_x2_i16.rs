//! Twice the area of a triangle from three signed vertices (x1,y1),(x2,y2),(x3,y3), via the same shoelace formula shoelace_area_x2 uses -- |x1*(y2-y3) + x2*(y3-y1) + x3*(y1-y2)| -- but over the full i16 plane instead of shoelace_area_x2's unsigned-only coordinates: every coordinate and every y-difference is tracked as a (magnitude, sign) pair via i16_mag/i16_neg (the technique orientation2d and geom_distance_3d already use for signed inputs), and each of the three terms is a full signed multiply of a signed coordinate by a signed difference, not just an unsigned coordinate times an unsigned difference.
//! tags: geometry, distance, area, triangle, shoelace, polygon, coordinate, signed, wide, u32, checked, escalate
//! entry: ShoelaceAreaX2I16::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if any difference, product, or running sum overflows u32
fn i16_mag(v: i16) -> u32 {
    if v < 0i16 { (0u16.wrapping_sub(v as u16)) as u32 } else { v as u16 as u32 }
}
fn i16_neg(v: i16) -> u16 {
    if v < 0i16 { 1u16 } else { 0u16 }
}
struct ShoelaceAreaX2I16 { x1: i16, y1: i16, x2: i16, y2: i16, x3: i16, y3: i16, result: u32 }
impl ShoelaceAreaX2I16 {
    fn run(&mut self) -> u16 {
        let y1_mag = i16_mag(self.y1);
        let y1_neg = i16_neg(self.y1);
        let y1_neg_f = 1u16 - y1_neg;
        let y2_mag = i16_mag(self.y2);
        let y2_neg = i16_neg(self.y2);
        let y2_neg_f = 1u16 - y2_neg;
        let y3_mag = i16_mag(self.y3);
        let y3_neg = i16_neg(self.y3);
        let y3_neg_f = 1u16 - y3_neg;

        // d1 = y2 - y3
        let mut d1_mag = 0u32;
        let mut d1_neg = 0u16;
        if y2_neg == y3_neg_f {
            d1_mag = add_checked_u32(y2_mag, y3_mag);
            d1_neg = y2_neg;
        } else if y2_mag >= y3_mag {
            d1_mag = y2_mag - y3_mag;
            d1_neg = if d1_mag == 0u32 { 0u16 } else { y2_neg };
        } else {
            d1_mag = y3_mag - y2_mag;
            d1_neg = y3_neg_f;
        }

        // d2 = y3 - y1
        let mut d2_mag = 0u32;
        let mut d2_neg = 0u16;
        if y3_neg == y1_neg_f {
            d2_mag = add_checked_u32(y3_mag, y1_mag);
            d2_neg = y3_neg;
        } else if y3_mag >= y1_mag {
            d2_mag = y3_mag - y1_mag;
            d2_neg = if d2_mag == 0u32 { 0u16 } else { y3_neg };
        } else {
            d2_mag = y1_mag - y3_mag;
            d2_neg = y1_neg_f;
        }

        // d3 = y1 - y2
        let mut d3_mag = 0u32;
        let mut d3_neg = 0u16;
        if y1_neg == y2_neg_f {
            d3_mag = add_checked_u32(y1_mag, y2_mag);
            d3_neg = y1_neg;
        } else if y1_mag >= y2_mag {
            d3_mag = y1_mag - y2_mag;
            d3_neg = if d3_mag == 0u32 { 0u16 } else { y1_neg };
        } else {
            d3_mag = y2_mag - y1_mag;
            d3_neg = y2_neg_f;
        }

        // term1 = x1 * d1 (signed * signed)
        let x1_mag = i16_mag(self.x1);
        let x1_neg = i16_neg(self.x1);
        let t1_mag = mul_checked_u32(x1_mag, d1_mag);
        let t1_neg = if t1_mag == 0u32 { 0u16 } else if x1_neg == d1_neg { 0u16 } else { 1u16 };

        // term2 = x2 * d2
        let x2_mag = i16_mag(self.x2);
        let x2_neg = i16_neg(self.x2);
        let t2_mag = mul_checked_u32(x2_mag, d2_mag);
        let t2_neg = if t2_mag == 0u32 { 0u16 } else if x2_neg == d2_neg { 0u16 } else { 1u16 };

        // term3 = x3 * d3
        let x3_mag = i16_mag(self.x3);
        let x3_neg = i16_neg(self.x3);
        let t3_mag = mul_checked_u32(x3_mag, d3_mag);
        let t3_neg = if t3_mag == 0u32 { 0u16 } else if x3_neg == d3_neg { 0u16 } else { 1u16 };

        // s12 = term1 + term2
        let mut s12_mag = 0u32;
        let mut s12_neg = 0u16;
        if t1_neg == t2_neg {
            s12_mag = add_checked_u32(t1_mag, t2_mag);
            s12_neg = t1_neg;
        } else if t1_mag >= t2_mag {
            s12_mag = t1_mag - t2_mag;
            s12_neg = if s12_mag == 0u32 { 0u16 } else { t1_neg };
        } else {
            s12_mag = t2_mag - t1_mag;
            s12_neg = t2_neg;
        }

        // final = s12 + term3
        let mut final_mag = 0u32;
        if s12_neg == t3_neg {
            final_mag = add_checked_u32(s12_mag, t3_mag);
        } else if s12_mag >= t3_mag {
            final_mag = s12_mag - t3_mag;
        } else {
            final_mag = t3_mag - s12_mag;
        }

        self.result = final_mag;
        1u16
    }
}
