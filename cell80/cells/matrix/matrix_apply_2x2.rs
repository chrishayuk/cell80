//! Apply a 2x2 matrix [[a, b], [c, d]] to a 2D vector (x, y): (rx, ry) = (a*x + b*y, c*x + d*y) -- the forward linear-transform counterpart matrix_solve_2x2's reverse Cramer's-rule solve has no companion for; each output combines two signed products via the same same-sign-add/opposite-sign-subtract sign-magnitude pattern matrix_det_2x2's own p1/p2 combine step uses.
//! tags: matrix, linear-algebra, 2x2, transform, apply, vector, wide, u32, checked, escalate
//! entry: MatrixApply2x2::run
//! limits: escalates (halt 0xFF05, needs_wider_math) on the (unreachable in practice for i16 inputs) intermediate overflow the shared add_checked_u32 kernel guards
fn i16_mag(v: i16) -> u32 {
    if v < 0i16 { (0u16.wrapping_sub(v as u16)) as u32 } else { v as u16 as u32 }
}
fn i16_neg(v: i16) -> u16 {
    if v < 0i16 { 1u16 } else { 0u16 }
}
struct MatrixApply2x2 {
    a: i16, b: i16, c: i16, d: i16, x: i16, y: i16,
    rx_mag: u32, rx_neg: u16, ry_mag: u32, ry_neg: u16
}
impl MatrixApply2x2 {
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
        let y_mag = i16_mag(self.y);
        let y_neg = i16_neg(self.y);

        // rx = a*x + b*y
        let p1_mag = a_mag * x_mag;
        let p1_neg = if a_neg == x_neg { 0u16 } else { 1u16 };
        let p2_mag = b_mag * y_mag;
        let p2_neg = if b_neg == y_neg { 0u16 } else { 1u16 };
        let mut rx_mag = 0u32;
        let mut rx_neg = 0u16;
        if p1_neg == p2_neg {
            rx_mag = add_checked_u32(p1_mag, p2_mag);
            rx_neg = p1_neg;
        } else if p1_mag >= p2_mag {
            rx_mag = p1_mag - p2_mag;
            rx_neg = p1_neg;
        } else {
            rx_mag = p2_mag - p1_mag;
            rx_neg = p2_neg;
        }
        if rx_mag == 0u32 { rx_neg = 0u16; }

        // ry = c*x + d*y
        let p3_mag = c_mag * x_mag;
        let p3_neg = if c_neg == x_neg { 0u16 } else { 1u16 };
        let p4_mag = d_mag * y_mag;
        let p4_neg = if d_neg == y_neg { 0u16 } else { 1u16 };
        let mut ry_mag = 0u32;
        let mut ry_neg = 0u16;
        if p3_neg == p4_neg {
            ry_mag = add_checked_u32(p3_mag, p4_mag);
            ry_neg = p3_neg;
        } else if p3_mag >= p4_mag {
            ry_mag = p3_mag - p4_mag;
            ry_neg = p3_neg;
        } else {
            ry_mag = p4_mag - p3_mag;
            ry_neg = p4_neg;
        }
        if ry_mag == 0u32 { ry_neg = 0u16; }

        self.rx_mag = rx_mag;
        self.rx_neg = rx_neg;
        self.ry_mag = ry_mag;
        self.ry_neg = ry_neg;
        1u16
    }
}
