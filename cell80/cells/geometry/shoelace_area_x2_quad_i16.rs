//! Twice the area of a quadrilateral from four signed vertices (x1,y1)..(x4,y4), the identical shoelace formula shoelace_area_x2_quad uses -- |x1*(y2-y4) + x2*(y3-y1) + x3*(y4-y2) + x4*(y1-y3)| -- but over signed i16 coordinates instead of unsigned u16, so every difference and every product is tracked as a sign-magnitude pair (never native i16 arithmetic), the same way shoelace_area_x2_i16 extends the unsigned triangle version to signed inputs.
//! tags: geometry, distance, area, quadrilateral, polygon, shoelace, coordinate, signed, i16, wide, u32, checked, escalate
//! entry: ShoelaceAreaX2QuadI16::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if any product or the running sum overflows u32
fn i16_mag(v: i16) -> u32 {
    if v < 0i16 { (0u16.wrapping_sub(v as u16)) as u32 } else { v as u16 as u32 }
}
fn i16_neg(v: i16) -> u16 {
    if v < 0i16 { 1u16 } else { 0u16 }
}
struct ShoelaceAreaX2QuadI16 { x1: i16, y1: i16, x2: i16, y2: i16, x3: i16, y3: i16, x4: i16, y4: i16, result: u32 }
impl ShoelaceAreaX2QuadI16 {
    fn run(&mut self) -> u16 {
        let x1_mag = i16_mag(self.x1);
        let x1_neg = i16_neg(self.x1);
        let y1_mag = i16_mag(self.y1);
        let y1_neg = i16_neg(self.y1);
        let x2_mag = i16_mag(self.x2);
        let x2_neg = i16_neg(self.x2);
        let y2_mag = i16_mag(self.y2);
        let y2_neg = i16_neg(self.y2);
        let x3_mag = i16_mag(self.x3);
        let x3_neg = i16_neg(self.x3);
        let y3_mag = i16_mag(self.y3);
        let y3_neg = i16_neg(self.y3);
        let x4_mag = i16_mag(self.x4);
        let x4_neg = i16_neg(self.x4);
        let y4_mag = i16_mag(self.y4);
        let y4_neg = i16_neg(self.y4);

        // d1 = y2 - y4
        let y4_neg_f1 = 1u16 - y4_neg;
        let mut mag_d1 = 0u32;
        let mut neg_d1 = 0u16;
        if y2_neg == y4_neg_f1 {
            mag_d1 = add_checked_u32(y2_mag, y4_mag);
            neg_d1 = y2_neg;
        } else if y2_mag >= y4_mag {
            mag_d1 = y2_mag - y4_mag;
            neg_d1 = y2_neg;
        } else {
            mag_d1 = y4_mag - y2_mag;
            neg_d1 = y4_neg_f1;
        }

        // d2 = y3 - y1
        let y1_neg_f2 = 1u16 - y1_neg;
        let mut mag_d2 = 0u32;
        let mut neg_d2 = 0u16;
        if y3_neg == y1_neg_f2 {
            mag_d2 = add_checked_u32(y3_mag, y1_mag);
            neg_d2 = y3_neg;
        } else if y3_mag >= y1_mag {
            mag_d2 = y3_mag - y1_mag;
            neg_d2 = y3_neg;
        } else {
            mag_d2 = y1_mag - y3_mag;
            neg_d2 = y1_neg_f2;
        }

        // d3 = y4 - y2
        let y2_neg_f3 = 1u16 - y2_neg;
        let mut mag_d3 = 0u32;
        let mut neg_d3 = 0u16;
        if y4_neg == y2_neg_f3 {
            mag_d3 = add_checked_u32(y4_mag, y2_mag);
            neg_d3 = y4_neg;
        } else if y4_mag >= y2_mag {
            mag_d3 = y4_mag - y2_mag;
            neg_d3 = y4_neg;
        } else {
            mag_d3 = y2_mag - y4_mag;
            neg_d3 = y2_neg_f3;
        }

        // d4 = y1 - y3
        let y3_neg_f4 = 1u16 - y3_neg;
        let mut mag_d4 = 0u32;
        let mut neg_d4 = 0u16;
        if y1_neg == y3_neg_f4 {
            mag_d4 = add_checked_u32(y1_mag, y3_mag);
            neg_d4 = y1_neg;
        } else if y1_mag >= y3_mag {
            mag_d4 = y1_mag - y3_mag;
            neg_d4 = y1_neg;
        } else {
            mag_d4 = y3_mag - y1_mag;
            neg_d4 = y3_neg_f4;
        }

        // t1 = x1 * d1
        let p1 = x1_mag.wrapping_mul(mag_d1);
        if x1_mag != 0u32 && p1 / x1_mag != mag_d1 { halt(0xFF05u16); }
        let mag_t1 = p1;
        let neg_t1 = if x1_neg == neg_d1 { 0u16 } else { 1u16 };

        // t2 = x2 * d2
        let p2 = x2_mag.wrapping_mul(mag_d2);
        if x2_mag != 0u32 && p2 / x2_mag != mag_d2 { halt(0xFF05u16); }
        let mag_t2 = p2;
        let neg_t2 = if x2_neg == neg_d2 { 0u16 } else { 1u16 };

        // t3 = x3 * d3
        let p3 = x3_mag.wrapping_mul(mag_d3);
        if x3_mag != 0u32 && p3 / x3_mag != mag_d3 { halt(0xFF05u16); }
        let mag_t3 = p3;
        let neg_t3 = if x3_neg == neg_d3 { 0u16 } else { 1u16 };

        // t4 = x4 * d4
        let p4 = x4_mag.wrapping_mul(mag_d4);
        if x4_mag != 0u32 && p4 / x4_mag != mag_d4 { halt(0xFF05u16); }
        let mag_t4 = p4;
        let neg_t4 = if x4_neg == neg_d4 { 0u16 } else { 1u16 };

        let mut mag_s = 0u32;
        let mut neg_s = 0u16;
        if neg_t1 == neg_t2 {
            let s = add_checked_u32(mag_t1, mag_t2);
            mag_s = s;
            neg_s = neg_t1;
        } else if mag_t1 >= mag_t2 {
            mag_s = mag_t1 - mag_t2;
            neg_s = neg_t1;
        } else {
            mag_s = mag_t2 - mag_t1;
            neg_s = neg_t2;
        }

        if neg_s == neg_t3 {
            let s = add_checked_u32(mag_s, mag_t3);
            mag_s = s;
            neg_s = neg_t3;
        } else if mag_s >= mag_t3 {
            mag_s = mag_s - mag_t3;
        } else {
            mag_s = mag_t3 - mag_s;
            neg_s = neg_t3;
        }

        let mut mag_final = 0u32;
        if neg_s == neg_t4 {
            let s = add_checked_u32(mag_s, mag_t4);
            mag_final = s;
        } else if mag_s >= mag_t4 {
            mag_final = mag_s - mag_t4;
        } else {
            mag_final = mag_t4 - mag_s;
        }

        self.result = mag_final;
        1u16
    }
}
