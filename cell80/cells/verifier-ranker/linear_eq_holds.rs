//! Verify a candidate x against a general one-variable linear equation a*x + b == c*x + d in one call -- the fused sibling of linear_solve_1var's solve step, exact via sign-magnitude arithmetic (no float tolerance), so a solved x round-trips through this check with zero error instead of an epsilon compare.
//! tags: linear, equation, verify, algebra, check, one-variable, signed, wide, u32, checked
//! entry: LinearEqHolds::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if an intermediate product-plus-addend sum would overflow u32 (not reachable for any i16 input; kept for the same checked-add honesty matrix_solve_2x2 uses)
fn i16_mag(v: i16) -> u32 {
    if v < 0i16 { (0u16.wrapping_sub(v as u16)) as u32 } else { v as u16 as u32 }
}
fn i16_neg(v: i16) -> u16 {
    if v < 0i16 { 1u16 } else { 0u16 }
}
struct LinearEqHolds { a: i16, b: i16, c: i16, d: i16, x: i16, ok: u16 }
impl LinearEqHolds {
    fn run(&mut self) -> u16 {
        let a_mag = i16_mag(self.a);
        let a_neg = i16_neg(self.a);
        let b_mag = i16_mag(self.b);
        let b_neg = i16_neg(self.b);
        let c_mag = i16_mag(self.c);
        let c_neg = i16_neg(self.c);
        let d_mag = i16_mag(self.d);
        let d_neg = i16_neg(self.d);
        let x_mag = i16_mag(self.x);
        let x_neg = i16_neg(self.x);

        // lhs = a*x + b
        let ax_mag = a_mag * x_mag;
        let ax_neg = if a_neg == x_neg { 0u16 } else { 1u16 };
        let mut lhs_mag = 0u32;
        let mut lhs_neg = 0u16;
        if ax_neg == b_neg {
            lhs_mag = add_checked_u32(ax_mag, b_mag);
            lhs_neg = ax_neg;
        } else if ax_mag >= b_mag {
            lhs_mag = ax_mag - b_mag;
            lhs_neg = if lhs_mag == 0u32 { 0u16 } else { ax_neg };
        } else {
            lhs_mag = b_mag - ax_mag;
            lhs_neg = b_neg;
        }

        // rhs = c*x + d
        let cx_mag = c_mag * x_mag;
        let cx_neg = if c_neg == x_neg { 0u16 } else { 1u16 };
        let mut rhs_mag = 0u32;
        let mut rhs_neg = 0u16;
        if cx_neg == d_neg {
            rhs_mag = add_checked_u32(cx_mag, d_mag);
            rhs_neg = cx_neg;
        } else if cx_mag >= d_mag {
            rhs_mag = cx_mag - d_mag;
            rhs_neg = if rhs_mag == 0u32 { 0u16 } else { cx_neg };
        } else {
            rhs_mag = d_mag - cx_mag;
            rhs_neg = d_neg;
        }

        self.ok = ((lhs_mag == rhs_mag) && (lhs_neg == rhs_neg)) as u16;
        self.ok
    }
}
