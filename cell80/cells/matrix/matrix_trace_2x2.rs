//! Trace of a 2x2 matrix [[a, _, _, d]]: a + d, the sum of the diagonal, tracked as a (magnitude, sign) pair -- matrix_det_2x2's own sibling invariant, the OTHER coefficient of the characteristic polynomial lambda^2 - trace*lambda + det = 0. Same-sign inputs combine via add_checked_u32 (checked, since a and d can each be i16::MIN/MAX); opposite-sign inputs subtract the smaller magnitude from the larger, sign following whichever operand had the larger magnitude -- plain i16 a + d could overflow i16's own range (e.g. a=d=i16::MAX), so this cannot be a native add.
//! tags: matrix, trace, linear-algebra, 2x2, characteristic-polynomial, signed, wide, u32, checked
//! entry: MatrixTrace2x2::run
fn i16_mag(v: i16) -> u32 {
    if v < 0i16 { (0u16.wrapping_sub(v as u16)) as u32 } else { v as u16 as u32 }
}
fn i16_neg(v: i16) -> u16 {
    if v < 0i16 { 1u16 } else { 0u16 }
}
struct MatrixTrace2x2 { a: i16, d: i16, trace_mag: u32, trace_neg: u16 }
impl MatrixTrace2x2 {
    fn run(&mut self) -> u16 {
        let a_mag = i16_mag(self.a);
        let a_neg = i16_neg(self.a);
        let d_mag = i16_mag(self.d);
        let d_neg = i16_neg(self.d);

        let mut trace_mag = 0u32;
        let mut trace_neg = 0u16;
        if a_neg == d_neg {
            trace_mag = add_checked_u32(a_mag, d_mag);
            trace_neg = a_neg;
        } else if a_mag >= d_mag {
            trace_mag = a_mag - d_mag;
            trace_neg = if trace_mag == 0u32 { 0u16 } else { a_neg };
        } else {
            trace_mag = d_mag - a_mag;
            trace_neg = if trace_mag == 0u32 { 0u16 } else { d_neg };
        }
        self.trace_mag = trace_mag;
        self.trace_neg = trace_neg;
        1u16
    }
}
