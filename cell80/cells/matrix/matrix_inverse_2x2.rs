//! Inverse of a 2x2 matrix [[a, b], [c, d]]: (1/det)*[[d, -b], [-c, a]] with det = a*d - b*c, all four output entries sharing one determinant division computed once and reused -- unlike matrix_solve_2x2 (an exact i16 signed numerator/denominator *fraction*, never actually dividing) and matrix_det_2x2/matrix_mul_2x2 (integer sign-magnitude, no division at all), this cell takes real f32 coefficients (matrix_solve_3x3's f32-throughout convention, extended here from a solved vector to a full inverse matrix) and performs the adjugate swap-and-negate then genuinely divides by det, escalating instead of returning a matrix for a singular or near-singular input.
//! tags: matrix, inverse, adjugate, linear-algebra, 2x2, determinant, f32, float, softfloat, escalate
//! kernel_bank: on
//! entry: MatrixInverse2x2::run
//! limits: escalates (halt 0xFF06, out_of_domain) if det is zero or within 1e-6 of zero (matrix_solve_3x3's own near-singular tolerance -- no unique inverse exists, matching Excel's #NUM! here); escalates (halt 0xFF08/0xFF07, float_domain/float_overflow) if any of the four divided entries goes NaN or non-finite before being written back
struct MatrixInverse2x2 {
    a: f32, b: f32, c: f32, d: f32,
    inv_a: f32, inv_b: f32, inv_c: f32, inv_d: f32,
}
impl MatrixInverse2x2 {
    fn run(&mut self) -> u16 {
        let det = self.a * self.d - self.b * self.c;
        let adet = det.abs();
        if adet < 0.000001f32 {
            halt(0xFF06u16);
        }

        let inv_a = self.d / det;
        let inv_b = -self.b / det;
        let inv_c = -self.c / det;
        let inv_d = self.a / det;

        if inv_a.is_nan() || inv_b.is_nan() || inv_c.is_nan() || inv_d.is_nan() {
            halt(0xFF08u16);
        }
        let fin1 = inv_a.is_finite();
        let fin2 = inv_b.is_finite();
        let fin3 = inv_c.is_finite();
        let fin4 = inv_d.is_finite();
        if !fin1 || !fin2 || !fin3 || !fin4 {
            halt(0xFF07u16);
        }

        self.inv_a = inv_a;
        self.inv_b = inv_b;
        self.inv_c = inv_c;
        self.inv_d = inv_d;
        1u16
    }
}
