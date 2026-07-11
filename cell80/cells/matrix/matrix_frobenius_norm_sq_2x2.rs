//! Squared Frobenius norm of a 2x2 matrix [[a, b], [c, d]]: a*a + b*b + c*c + d*d, widened to a u32 field -- the matrix pack's own third orthogonal scalar invariant alongside matrix_det_2x2's signed area-scaling and matrix_trace_2x2's diagonal sum, transplanting the vector pack's norm2_sq/norm3_sq pattern onto the matrix's 4 elements treated as a flat vector. Each element's square is always non-negative, so this tracks magnitude only via i16_mag and never needs the sign-combining step matrix_det_2x2/matrix_trace_2x2 require for their differences or sums of signed products.
//! tags: matrix, frobenius, norm, magnitude, squared, 2x2, invariant, signed, wide, u32, checked
//! entry: MatrixFrobeniusNormSq2x2::run
//! limits: escalates (halt 0xFF05, needs_wider_math) on the (unreachable in practice for i16 inputs) intermediate overflow the shared mul_checked_u32/add_checked_u32 kernels guard
fn i16_mag(v: i16) -> u32 {
    if v < 0i16 { (0u16.wrapping_sub(v as u16)) as u32 } else { v as u16 as u32 }
}
struct MatrixFrobeniusNormSq2x2 { a: i16, b: i16, c: i16, d: i16, result: u32 }
impl MatrixFrobeniusNormSq2x2 {
    fn run(&mut self) -> u16 {
        let a_mag = i16_mag(self.a);
        let b_mag = i16_mag(self.b);
        let c_mag = i16_mag(self.c);
        let d_mag = i16_mag(self.d);

        let a_sq = mul_checked_u32(a_mag, a_mag);
        let b_sq = mul_checked_u32(b_mag, b_mag);
        let c_sq = mul_checked_u32(c_mag, c_mag);
        let d_sq = mul_checked_u32(d_mag, d_mag);

        let sum1 = add_checked_u32(a_sq, b_sq);
        let sum2 = add_checked_u32(sum1, c_sq);
        let sum3 = add_checked_u32(sum2, d_sq);

        self.result = sum3;
        1u16
    }
}
