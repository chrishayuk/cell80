//! Cross product of two 3D vectors: (ay*bz - az*by, az*bx - ax*bz, ax*by - ay*bx). Each signed component is tracked as a (magnitude, sign) pair through the multiply and the combining subtract -- the same technique vectors_parallel uses for its equality checks, extended one step further here since a real signed result (not just a zero/nonzero check) is needed. The result can exceed either input's own magnitude, so it rides wide u32-magnitude output fields rather than being narrowed back to i16.
//! tags: vector, cross-product, geometry, 3d, wide, u32, checked, escalate
//! entry: CrossProduct::run
//! limits: escalates (halt 0xFF05, needs_wider_math) on the (unreachable in practice for i16 inputs) intermediate overflow the shared add_checked_u32 kernel guards
fn i16_mag(v: i16) -> u32 {
    if v < 0i16 { (0u16.wrapping_sub(v as u16)) as u32 } else { v as u16 as u32 }
}
fn i16_neg(v: i16) -> u16 {
    if v < 0i16 { 1u16 } else { 0u16 }
}
struct CrossProduct {
    ax: i16, ay: i16, az: i16, bx: i16, by: i16, bz: i16,
    rx_mag: u32, rx_neg: u16, ry_mag: u32, ry_neg: u16, rz_mag: u32, rz_neg: u16
}
impl CrossProduct {
    fn run(&mut self) -> u16 {
        let ax_mag = i16_mag(self.ax);
        let ax_neg = i16_neg(self.ax);
        let ay_mag = i16_mag(self.ay);
        let ay_neg = i16_neg(self.ay);
        let az_mag = i16_mag(self.az);
        let az_neg = i16_neg(self.az);
        let bx_mag = i16_mag(self.bx);
        let bx_neg = i16_neg(self.bx);
        let by_mag = i16_mag(self.by);
        let by_neg = i16_neg(self.by);
        let bz_mag = i16_mag(self.bz);
        let bz_neg = i16_neg(self.bz);

        // rx = ay*bz - az*by
        let p1_mag = ay_mag * bz_mag;
        let p1_neg = if ay_neg == bz_neg { 0u16 } else { 1u16 };
        let p2_mag = az_mag * by_mag;
        let p2_neg = if az_neg == by_neg { 0u16 } else { 1u16 };
        let p2_neg_f = if p2_neg == 0u16 { 1u16 } else { 0u16 };
        let mut rx_mag = 0u32;
        let mut rx_neg = 0u16;
        if p1_neg == p2_neg_f {
            rx_mag = add_checked_u32(p1_mag, p2_mag);
            rx_neg = p1_neg;
        } else if p1_mag >= p2_mag {
            rx_mag = p1_mag - p2_mag;
            rx_neg = p1_neg;
        } else {
            rx_mag = p2_mag - p1_mag;
            rx_neg = p2_neg_f;
        }

        // ry = az*bx - ax*bz
        let p3_mag = az_mag * bx_mag;
        let p3_neg = if az_neg == bx_neg { 0u16 } else { 1u16 };
        let p4_mag = ax_mag * bz_mag;
        let p4_neg = if ax_neg == bz_neg { 0u16 } else { 1u16 };
        let p4_neg_f = if p4_neg == 0u16 { 1u16 } else { 0u16 };
        let mut ry_mag = 0u32;
        let mut ry_neg = 0u16;
        if p3_neg == p4_neg_f {
            ry_mag = add_checked_u32(p3_mag, p4_mag);
            ry_neg = p3_neg;
        } else if p3_mag >= p4_mag {
            ry_mag = p3_mag - p4_mag;
            ry_neg = p3_neg;
        } else {
            ry_mag = p4_mag - p3_mag;
            ry_neg = p4_neg_f;
        }

        // rz = ax*by - ay*bx
        let p5_mag = ax_mag * by_mag;
        let p5_neg = if ax_neg == by_neg { 0u16 } else { 1u16 };
        let p6_mag = ay_mag * bx_mag;
        let p6_neg = if ay_neg == bx_neg { 0u16 } else { 1u16 };
        let p6_neg_f = if p6_neg == 0u16 { 1u16 } else { 0u16 };
        let mut rz_mag = 0u32;
        let mut rz_neg = 0u16;
        if p5_neg == p6_neg_f {
            rz_mag = add_checked_u32(p5_mag, p6_mag);
            rz_neg = p5_neg;
        } else if p5_mag >= p6_mag {
            rz_mag = p5_mag - p6_mag;
            rz_neg = p5_neg;
        } else {
            rz_mag = p6_mag - p5_mag;
            rz_neg = p6_neg_f;
        }

        self.rx_mag = rx_mag;
        self.rx_neg = rx_neg;
        self.ry_mag = ry_mag;
        self.ry_neg = ry_neg;
        self.rz_mag = rz_mag;
        self.rz_neg = rz_neg;
        1u16
    }
}
