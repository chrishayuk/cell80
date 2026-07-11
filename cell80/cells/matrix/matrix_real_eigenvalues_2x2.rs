//! Predicate: does the 2x2 matrix [[a, b], [c, d]] have two real eigenvalues (as opposed to a complex-conjugate pair)? Recomputes trace = a+d and det = a*d - b*c inline (each sign-magnitude, the same technique matrix_trace_2x2/matrix_det_2x2 use, inlined here the way matrix_solve_2x2 inlines matrix_det_2x2's own formula) then forms the characteristic-polynomial discriminant = trace^2 - 4*det and returns (discriminant >= 0) as 0/1 -- deliberately stops at this reality predicate rather than an sqrt of a possibly-negative quantity.
//! tags: matrix, eigenvalues, discriminant, characteristic-polynomial, predicate, 2x2, linear-algebra, trace, determinant, sign-magnitude, wide, u32, checked, escalate
//! entry: MatrixRealEigenvalues2x2::run
//! limits: escalates (halt 0xFF05, needs_wider_math) on intermediate overflow when squaring the trace magnitude or scaling the determinant magnitude by 4 (via the shared mul_checked_u32/add_checked_u32 kernels)
fn i16_mag(v: i16) -> u32 {
    if v < 0i16 { (0u16.wrapping_sub(v as u16)) as u32 } else { v as u16 as u32 }
}
fn i16_neg(v: i16) -> u16 {
    if v < 0i16 { 1u16 } else { 0u16 }
}
struct MatrixRealEigenvalues2x2 { a: i16, b: i16, c: i16, d: i16, result: u16 }
impl MatrixRealEigenvalues2x2 {
    fn run(&mut self) -> u16 {
        let a_mag = i16_mag(self.a);
        let a_neg = i16_neg(self.a);
        let b_mag = i16_mag(self.b);
        let b_neg = i16_neg(self.b);
        let c_mag = i16_mag(self.c);
        let c_neg = i16_neg(self.c);
        let d_mag = i16_mag(self.d);
        let d_neg = i16_neg(self.d);

        // trace = a + d (magnitude only -- its sign doesn't matter once squared below)
        let mut trace_mag = 0u32;
        if a_neg == d_neg {
            trace_mag = add_checked_u32(a_mag, d_mag);
        } else if a_mag >= d_mag {
            trace_mag = a_mag - d_mag;
        } else {
            trace_mag = d_mag - a_mag;
        }

        // det = a*d - b*c, the same sign-magnitude combine matrix_det_2x2 uses.
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
        if det_mag == 0u32 { det_neg = 0u16; }

        // discriminant = trace^2 - 4*det; only its sign matters for this predicate.
        let trace_sq_mag = mul_checked_u32(trace_mag, trace_mag);
        let four_det_mag = mul_checked_u32(det_mag, 4u32);
        let mut disc_neg = 0u16;
        if det_neg == 1u16 {
            // det is negative, so -4*det is a non-negative addend -> discriminant >= 0 always.
            disc_neg = 0u16;
        } else if trace_sq_mag >= four_det_mag {
            disc_neg = 0u16;
        } else {
            disc_neg = 1u16;
        }

        let result = (disc_neg == 0u16) as u16;
        self.result = result;
        1u16
    }
}
