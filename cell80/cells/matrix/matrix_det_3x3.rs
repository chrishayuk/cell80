//! Determinant of a fixed 3x3 matrix via the standard first-row cofactor expansion: det = a11*(a22*a33-a23*a32) - a12*(a21*a33-a23*a31) + a13*(a21*a32-a22*a31), the same three column-1 minors and the same expansion matrix_solve_3x3 already builds and divides by internally -- unlike that cell, which only ever uses this value as Cramer's-rule divisor (and halts if it is near zero), this cell returns the bare determinant as its own standalone result, with no RHS vector, no division, and no near-singular escalation, since a determinant of exactly 0 (a singular matrix) is itself a legitimate answer here. This is Excel's MDETERM, landed in the "matrix" pack (not excel-mathstat) per this wave's naming convention, matching its 2x2 sibling matrix_det_2x2.
//! tags: excel, mdeterm, matrix, determinant, linear-algebra, 3x3, square-matrix, cofactor, cofactor-expansion, minor, f32, float, softfloat
//! kernel_bank: on
//! entry: MatrixDet3x3::run
//! limits: fixed at exactly 3x3 (9 named coefficients a11..a33, no general NxN capability); escalates (halt 0xFF08, float_domain) if the result is NaN, or (halt 0xFF07, float_overflow) if it's non-finite; unlike matrix_solve_3x3, a zero (or near-zero) determinant is NOT an error -- Excel's MDETERM returns 0 outright for a singular matrix, so this cell reports it directly with no threshold check
struct MatrixDet3x3 {
    a11: f32, a12: f32, a13: f32,
    a21: f32, a22: f32, a23: f32,
    a31: f32, a32: f32, a33: f32,
    det: f32,
}
impl MatrixDet3x3 {
    fn run(&mut self) -> u16 {
        // The three minors of column 1 -- matrix_solve_3x3's own m11/m12/m13, reused
        // here for the determinant alone rather than as shared terms across four
        // Cramer's-rule numerators.
        let m11 = self.a22 * self.a33 - self.a23 * self.a32;
        let m12 = self.a21 * self.a33 - self.a23 * self.a31;
        let m13 = self.a21 * self.a32 - self.a22 * self.a31;

        let det = self.a11 * m11 - self.a12 * m12 + self.a13 * m13;

        if det.is_nan() {
            halt(0xFF08u16);
        }
        let fin = det.is_finite();
        if !fin {
            halt(0xFF07u16);
        }

        self.det = det;
        1u16
    }
}
