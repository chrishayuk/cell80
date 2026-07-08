//! Solve a 2x2 linear system [[a, b], [c, d]] * [x, y] = [e, f] via Cramer's rule, returning x and y as exact signed fractions sharing one positive denominator (det, normalized positive by flipping both numerators' signs if the raw determinant was negative) -- matrix_det_2x2's own formula computes that shared denominator's magnitude and sign before this cell reuses it inline.
//! tags: matrix, linear-algebra, 2x2, solve, cramers-rule, system, equations, fraction, wide, u32, checked, escalate
//! entry: MatrixSolve2x2::run
//! limits: escalates (halt 0xFF06, out_of_domain) if the determinant is zero (no unique solution); escalates (halt 0xFF05, needs_wider_math) on intermediate overflow
fn i16_mag(v: i16) -> u32 {
    if v < 0i16 { (0u16.wrapping_sub(v as u16)) as u32 } else { v as u16 as u32 }
}
fn i16_neg(v: i16) -> u16 {
    if v < 0i16 { 1u16 } else { 0u16 }
}
struct MatrixSolve2x2 {
    a: i16, b: i16, c: i16, d: i16, e: i16, f: i16,
    x_num_mag: u32, x_num_neg: u16, y_num_mag: u32, y_num_neg: u16, den: u32
}
impl MatrixSolve2x2 {
    fn run(&mut self) -> u16 {
        let a_mag = i16_mag(self.a);
        let a_neg = i16_neg(self.a);
        let b_mag = i16_mag(self.b);
        let b_neg = i16_neg(self.b);
        let c_mag = i16_mag(self.c);
        let c_neg = i16_neg(self.c);
        let d_mag = i16_mag(self.d);
        let d_neg = i16_neg(self.d);
        let e_mag = i16_mag(self.e);
        let e_neg = i16_neg(self.e);
        let f_mag = i16_mag(self.f);
        let f_neg = i16_neg(self.f);

        // det = a*d - b*c
        let p1_mag = a_mag * d_mag;
        let p1_neg = if a_neg == d_neg { 0u16 } else { 1u16 };
        let p2_mag = b_mag * c_mag;
        let p2_neg = if b_neg == c_neg { 0u16 } else { 1u16 };
        let p2_neg_f = if p2_neg == 0u16 { 1u16 } else { 0u16 };
        let mut det_mag = 0u32;
        let mut det_neg = 0u16;
        if p1_neg == p2_neg_f {
            det_mag = add_checked_u32(p1_mag, p2_mag);
            det_neg = p1_neg;
        } else if p1_mag >= p2_mag {
            det_mag = p1_mag - p2_mag;
            det_neg = p1_neg;
        } else {
            det_mag = p2_mag - p1_mag;
            det_neg = p2_neg_f;
        }
        if det_mag == 0u32 { halt(0xFF06u16); }

        // x_num = e*d - b*f
        let p3_mag = e_mag * d_mag;
        let p3_neg = if e_neg == d_neg { 0u16 } else { 1u16 };
        let p4_mag = b_mag * f_mag;
        let p4_neg = if b_neg == f_neg { 0u16 } else { 1u16 };
        let p4_neg_f = if p4_neg == 0u16 { 1u16 } else { 0u16 };
        let mut x_num_mag = 0u32;
        let mut x_num_neg = 0u16;
        if p3_neg == p4_neg_f {
            x_num_mag = add_checked_u32(p3_mag, p4_mag);
            x_num_neg = p3_neg;
        } else if p3_mag >= p4_mag {
            x_num_mag = p3_mag - p4_mag;
            x_num_neg = p3_neg;
        } else {
            x_num_mag = p4_mag - p3_mag;
            x_num_neg = p4_neg_f;
        }

        // y_num = a*f - e*c
        let p5_mag = a_mag * f_mag;
        let p5_neg = if a_neg == f_neg { 0u16 } else { 1u16 };
        let p6_mag = e_mag * c_mag;
        let p6_neg = if e_neg == c_neg { 0u16 } else { 1u16 };
        let p6_neg_f = if p6_neg == 0u16 { 1u16 } else { 0u16 };
        let mut y_num_mag = 0u32;
        let mut y_num_neg = 0u16;
        if p5_neg == p6_neg_f {
            y_num_mag = add_checked_u32(p5_mag, p6_mag);
            y_num_neg = p5_neg;
        } else if p5_mag >= p6_mag {
            y_num_mag = p5_mag - p6_mag;
            y_num_neg = p5_neg;
        } else {
            y_num_mag = p6_mag - p5_mag;
            y_num_neg = p6_neg_f;
        }

        // Normalize so the shared denominator is positive: if det was negative,
        // flip both numerators' signs (equivalent fraction, positive denominator).
        let mut x_final_neg = x_num_neg;
        let mut y_final_neg = y_num_neg;
        if det_neg == 1u16 {
            x_final_neg = if x_num_neg == 0u16 { 1u16 } else { 0u16 };
            y_final_neg = if y_num_neg == 0u16 { 1u16 } else { 0u16 };
        }
        if x_num_mag == 0u32 { x_final_neg = 0u16; }
        if y_num_mag == 0u32 { y_final_neg = 0u16; }

        self.x_num_mag = x_num_mag;
        self.x_num_neg = x_final_neg;
        self.y_num_mag = y_num_mag;
        self.y_num_neg = y_final_neg;
        self.den = det_mag;
        1u16
    }
}
