//! Multiply two 2x2 matrices A=[[a,b],[c,d]] and B=[[e,f],[g,h]] to get C=A*B: c11=a*e+b*g, c12=a*f+b*h, c21=c*e+d*g, c22=c*f+d*h, each entry a signed sum of two products tracked as a (magnitude, sign) pair via the same same-sign-add(add_checked_u32)/opposite-sign-subtract technique matrix_det_2x2 establishes -- composing two linear transforms, distinct from matrix_det_2x2 (a single scalar subtraction) and matrix_solve_2x2 (Cramer's-rule division into a fraction): this is pure signed multiply-accumulate, no division, no domain-error escalation.
//! tags: matrix, multiply, linear-algebra, 2x2, compose, transform, signed, wide, u32, checked, escalate
//! entry: MatrixMul2x2::run
//! limits: escalates (halt 0xFF05, needs_wider_math) on the (unreachable in practice for i16 inputs) intermediate overflow the shared add_checked_u32 kernel guards
fn i16_mag(v: i16) -> u32 {
    if v < 0i16 { (0u16.wrapping_sub(v as u16)) as u32 } else { v as u16 as u32 }
}
fn i16_neg(v: i16) -> u16 {
    if v < 0i16 { 1u16 } else { 0u16 }
}
struct MatrixMul2x2 {
    a: i16, b: i16, c: i16, d: i16, e: i16, f: i16, g: i16, h: i16,
    r11_mag: u32, r11_neg: u16, r12_mag: u32, r12_neg: u16,
    r21_mag: u32, r21_neg: u16, r22_mag: u32, r22_neg: u16
}
impl MatrixMul2x2 {
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
        let g_mag = i16_mag(self.g);
        let g_neg = i16_neg(self.g);
        let h_mag = i16_mag(self.h);
        let h_neg = i16_neg(self.h);

        // r11 = a*e + b*g
        let p1_mag = a_mag * e_mag;
        let p1_neg = if a_neg == e_neg { 0u16 } else { 1u16 };
        let p2_mag = b_mag * g_mag;
        let p2_neg = if b_neg == g_neg { 0u16 } else { 1u16 };
        let mut r11_mag = 0u32;
        let mut r11_neg = 0u16;
        if p1_neg == p2_neg {
            r11_mag = add_checked_u32(p1_mag, p2_mag);
            r11_neg = p1_neg;
        } else if p1_mag >= p2_mag {
            r11_mag = p1_mag - p2_mag;
            r11_neg = if r11_mag == 0u32 { 0u16 } else { p1_neg };
        } else {
            r11_mag = p2_mag - p1_mag;
            r11_neg = if r11_mag == 0u32 { 0u16 } else { p2_neg };
        }

        // r12 = a*f + b*h
        let p3_mag = a_mag * f_mag;
        let p3_neg = if a_neg == f_neg { 0u16 } else { 1u16 };
        let p4_mag = b_mag * h_mag;
        let p4_neg = if b_neg == h_neg { 0u16 } else { 1u16 };
        let mut r12_mag = 0u32;
        let mut r12_neg = 0u16;
        if p3_neg == p4_neg {
            r12_mag = add_checked_u32(p3_mag, p4_mag);
            r12_neg = p3_neg;
        } else if p3_mag >= p4_mag {
            r12_mag = p3_mag - p4_mag;
            r12_neg = if r12_mag == 0u32 { 0u16 } else { p3_neg };
        } else {
            r12_mag = p4_mag - p3_mag;
            r12_neg = if r12_mag == 0u32 { 0u16 } else { p4_neg };
        }

        // r21 = c*e + d*g
        let p5_mag = c_mag * e_mag;
        let p5_neg = if c_neg == e_neg { 0u16 } else { 1u16 };
        let p6_mag = d_mag * g_mag;
        let p6_neg = if d_neg == g_neg { 0u16 } else { 1u16 };
        let mut r21_mag = 0u32;
        let mut r21_neg = 0u16;
        if p5_neg == p6_neg {
            r21_mag = add_checked_u32(p5_mag, p6_mag);
            r21_neg = p5_neg;
        } else if p5_mag >= p6_mag {
            r21_mag = p5_mag - p6_mag;
            r21_neg = if r21_mag == 0u32 { 0u16 } else { p5_neg };
        } else {
            r21_mag = p6_mag - p5_mag;
            r21_neg = if r21_mag == 0u32 { 0u16 } else { p6_neg };
        }

        // r22 = c*f + d*h
        let p7_mag = c_mag * f_mag;
        let p7_neg = if c_neg == f_neg { 0u16 } else { 1u16 };
        let p8_mag = d_mag * h_mag;
        let p8_neg = if d_neg == h_neg { 0u16 } else { 1u16 };
        let mut r22_mag = 0u32;
        let mut r22_neg = 0u16;
        if p7_neg == p8_neg {
            r22_mag = add_checked_u32(p7_mag, p8_mag);
            r22_neg = p7_neg;
        } else if p7_mag >= p8_mag {
            r22_mag = p7_mag - p8_mag;
            r22_neg = if r22_mag == 0u32 { 0u16 } else { p7_neg };
        } else {
            r22_mag = p8_mag - p7_mag;
            r22_neg = if r22_mag == 0u32 { 0u16 } else { p8_neg };
        }

        self.r11_mag = r11_mag;
        self.r11_neg = r11_neg;
        self.r12_mag = r12_mag;
        self.r12_neg = r12_neg;
        self.r21_mag = r21_mag;
        self.r21_neg = r21_neg;
        self.r22_mag = r22_mag;
        self.r22_neg = r22_neg;
        1u16
    }
}
