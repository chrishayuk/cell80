//! Solve a general one-variable linear equation a*x + b = c*x + d for x, returned as an exact signed fraction (num_mag/num_neg over a positive den) in lowest terms via the shared gcd_u32 kernel -- the single-unknown sibling of matrix_solve_2x2's two-unknown Cramer's-rule solve. num = d - b and den = a - c are plain signed subtractions, not products, so this needs sign-magnitude tracking (the dialect has no i32 yet) but no overflow-prone multiply.
//! tags: linear, equation, solve, algebra, one-variable, fraction, signed, wide, u32, checked, escalate
//! entry: LinearSolve1Var::run
//! limits: escalates (halt 0xFF06, out_of_domain) if a == c (no unique solution: either no solution or infinitely many)
fn i16_mag(v: i16) -> u32 {
    if v < 0i16 { (0u16.wrapping_sub(v as u16)) as u32 } else { v as u16 as u32 }
}
fn i16_neg(v: i16) -> u16 {
    if v < 0i16 { 1u16 } else { 0u16 }
}
struct LinearSolve1Var { a: i16, b: i16, c: i16, d: i16, num_mag: u32, num_neg: u16, den: u32 }
impl LinearSolve1Var {
    fn run(&mut self) -> u16 {
        // num = d - b
        let d_mag = i16_mag(self.d);
        let d_neg = i16_neg(self.d);
        let b_mag = i16_mag(self.b);
        let b_neg_f = 1u16 - i16_neg(self.b);
        let mut num_mag = 0u32;
        let mut num_neg = 0u16;
        if d_neg == b_neg_f {
            num_mag = add_checked_u32(d_mag, b_mag);
            num_neg = d_neg;
        } else if d_mag >= b_mag {
            num_mag = d_mag - b_mag;
            num_neg = if num_mag == 0u32 { 0u16 } else { d_neg };
        } else {
            num_mag = b_mag - d_mag;
            num_neg = b_neg_f;
        }

        // den = a - c
        let a_mag = i16_mag(self.a);
        let a_neg = i16_neg(self.a);
        let c_mag = i16_mag(self.c);
        let c_neg_f = 1u16 - i16_neg(self.c);
        let mut den_mag = 0u32;
        let mut den_neg = 0u16;
        if a_neg == c_neg_f {
            den_mag = add_checked_u32(a_mag, c_mag);
            den_neg = a_neg;
        } else if a_mag >= c_mag {
            den_mag = a_mag - c_mag;
            den_neg = if den_mag == 0u32 { 0u16 } else { a_neg };
        } else {
            den_mag = c_mag - a_mag;
            den_neg = c_neg_f;
        }
        if den_mag == 0u32 { halt(0xFF06u16); }

        // Normalize so the denominator is positive: flip the numerator's sign if den was negative.
        let mut final_num_neg = num_neg;
        if den_neg == 1u16 { final_num_neg = 1u16 - num_neg; }
        if num_mag == 0u32 { final_num_neg = 0u16; }

        let g = gcd_u32(num_mag, den_mag);
        self.num_mag = num_mag / g;
        self.num_neg = final_num_neg;
        self.den = den_mag / g;
        1u16
    }
}
