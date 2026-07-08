//! Determinant of a 2x2 matrix [[a, b], [c, d]]: a*d - b*c. Signed result tracked as a (magnitude, sign) pair, the same technique the vector pack's cross_product/triple_scalar_product use -- the "vector floor" exception to the matrix non-goal extends this far and no further (see docs/library-growth.md).
//! tags: matrix, determinant, linear-algebra, 2x2, wide, u32, checked, escalate
//! entry: MatrixDet2x2::run
//! limits: escalates (halt 0xFF05, needs_wider_math) on the (unreachable in practice for i16 inputs) intermediate overflow the shared add_checked_u32 kernel guards
fn i16_mag(v: i16) -> u32 {
    if v < 0i16 { (0u16.wrapping_sub(v as u16)) as u32 } else { v as u16 as u32 }
}
fn i16_neg(v: i16) -> u16 {
    if v < 0i16 { 1u16 } else { 0u16 }
}
struct MatrixDet2x2 { a: i16, b: i16, c: i16, d: i16, result_mag: u32, result_neg: u16 }
impl MatrixDet2x2 {
    fn run(&mut self) -> u16 {
        let a_mag = i16_mag(self.a);
        let a_neg = i16_neg(self.a);
        let d_mag = i16_mag(self.d);
        let d_neg = i16_neg(self.d);
        let b_mag = i16_mag(self.b);
        let b_neg = i16_neg(self.b);
        let c_mag = i16_mag(self.c);
        let c_neg = i16_neg(self.c);

        let p1_mag = a_mag * d_mag;
        let p1_neg = if a_neg == d_neg { 0u16 } else { 1u16 };
        let p2_mag = b_mag * c_mag;
        let p2_neg = if b_neg == c_neg { 0u16 } else { 1u16 };
        let p2_neg_f = if p2_neg == 0u16 { 1u16 } else { 0u16 };
        let mut r_mag = 0u32;
        let mut r_neg = 0u16;
        if p1_neg == p2_neg_f {
            r_mag = add_checked_u32(p1_mag, p2_mag);
            r_neg = p1_neg;
        } else if p1_mag >= p2_mag {
            r_mag = p1_mag - p2_mag;
            r_neg = p1_neg;
        } else {
            r_mag = p2_mag - p1_mag;
            r_neg = p2_neg_f;
        }
        self.result_mag = r_mag;
        self.result_neg = r_neg;
        1u16
    }
}
