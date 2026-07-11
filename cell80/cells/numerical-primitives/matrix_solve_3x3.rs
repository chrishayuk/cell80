//! Solve a 3x3 linear system Ax=b via Cramer's rule, extending matrix_solve_2x2's cofactor-free 2x2 fraction to one more dimension: 9 named f32 coefficients a11..a33 plus 3 RHS values b1..b3, each x_i formed as det(A with column i replaced by b)/det(A) via the standard 3x3 cofactor expansion (the three column-1 minors of A are computed once and reused across all four determinants) -- unlike matrix_solve_2x2's exact signed-fraction (a native i16 2x2 cross-multiply never overflows u32), a 3x3 expansion multiplies three terms deep, which would blow past even a widened integer kernel, so this cell is IEEE binary32 throughout and returns real (possibly inexact) quotients rather than an exact numerator/denominator pair.
//! tags: matrix, linear-algebra, 3x3, solve, cramers-rule, system, equations, cofactor, cofactor-expansion, determinant, three-variable, simultaneous-equations, f32, float, softfloat, escalate
//! kernel_bank: on
//! entry: MatrixSolve3x3::run
//! limits: escalates (halt 0xFF06, out_of_domain) if det(A) is zero or within 1e-6 of zero (no unique solution, or too ill-conditioned to trust in binary32); escalates (halt 0xFF08, float_domain) if any solved x_i is NaN; escalates (halt 0xFF07, float_overflow) if any solved x_i is non-finite
struct MatrixSolve3x3 {
    a11: f32, a12: f32, a13: f32,
    a21: f32, a22: f32, a23: f32,
    a31: f32, a32: f32, a33: f32,
    b1: f32, b2: f32, b3: f32,
    x1: f32, x2: f32, x3: f32,
}
impl MatrixSolve3x3 {
    fn run(&mut self) -> u16 {
        // The three minors of column 1, shared by det(A) and every replaced-column
        // determinant below (each replaces a different column, so column 1's own
        // minors -- which never touch column 1 -- carry over unchanged).
        let m11 = self.a22 * self.a33 - self.a23 * self.a32;
        let m12 = self.a21 * self.a33 - self.a23 * self.a31;
        let m13 = self.a21 * self.a32 - self.a22 * self.a31;

        let det = self.a11 * m11 - self.a12 * m12 + self.a13 * m13;

        let adet = det.abs();
        if adet < 0.000001f32 {
            halt(0xFF06u16);
        }

        // Terms mixing b into columns 2/3, shared across the three replaced-column
        // determinants the same way m11/m12/m13 are shared above.
        let n1 = self.b2 * self.a33 - self.a23 * self.b3;
        let n2 = self.b2 * self.a32 - self.a22 * self.b3;
        let n3 = self.a21 * self.b3 - self.b2 * self.a31;

        let det_x1 = self.b1 * m11 - self.a12 * n1 + self.a13 * n2;
        let det_x2 = self.a11 * n1 - self.b1 * m12 + self.a13 * n3;
        let det_x3 = -(self.a11 * n2) - self.a12 * n3 + self.b1 * m13;

        let x1 = det_x1 / det;
        let x2 = det_x2 / det;
        let x3 = det_x3 / det;

        if x1.is_nan() || x2.is_nan() || x3.is_nan() {
            halt(0xFF08u16);
        }
        let fin1 = x1.is_finite();
        let fin2 = x2.is_finite();
        let fin3 = x3.is_finite();
        if !fin1 || !fin2 || !fin3 {
            halt(0xFF07u16);
        }

        self.x1 = x1;
        self.x2 = x2;
        self.x3 = x3;
        1u16
    }
}
