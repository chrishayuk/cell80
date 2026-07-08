//! Triple scalar product a . (b x c) of three 3D vectors -- the signed volume of the parallelepiped they span (zero exactly when the three vectors are coplanar). Computed as cross_product(b, c) followed by a signed dot with a, reusing the same (magnitude, sign) tracking cross_product and vectors_parallel already establish, so no new arithmetic technique is introduced here.
//! tags: vector, triple-product, scalar-triple-product, determinant, volume, coplanar, geometry, 3d, wide, u32, checked, escalate
//! entry: TripleScalarProduct::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if an intermediate product or sum overflows u32
fn i16_mag(v: i16) -> u32 {
    if v < 0i16 { (0u16.wrapping_sub(v as u16)) as u32 } else { v as u16 as u32 }
}
fn i16_neg(v: i16) -> u16 {
    if v < 0i16 { 1u16 } else { 0u16 }
}
struct TripleScalarProduct {
    ax: i16, ay: i16, az: i16,
    bx: i16, by: i16, bz: i16,
    cx: i16, cy: i16, cz: i16,
    result_mag: u32, result_neg: u16
}
impl TripleScalarProduct {
    fn run(&mut self) -> u16 {
        let bx_mag = i16_mag(self.bx);
        let bx_neg = i16_neg(self.bx);
        let by_mag = i16_mag(self.by);
        let by_neg = i16_neg(self.by);
        let bz_mag = i16_mag(self.bz);
        let bz_neg = i16_neg(self.bz);
        let cx_mag = i16_mag(self.cx);
        let cx_neg = i16_neg(self.cx);
        let cy_mag = i16_mag(self.cy);
        let cy_neg = i16_neg(self.cy);
        let cz_mag = i16_mag(self.cz);
        let cz_neg = i16_neg(self.cz);

        // cross(b, c) = (by*cz - bz*cy, bz*cx - bx*cz, bx*cy - by*cx)
        let p1_mag = mul_checked_u32(by_mag, cz_mag);
        let p1_neg = if by_neg == cz_neg { 0u16 } else { 1u16 };
        let p2_mag = mul_checked_u32(bz_mag, cy_mag);
        let p2_neg = if bz_neg == cy_neg { 0u16 } else { 1u16 };
        let p2_neg_f = if p2_neg == 0u16 { 1u16 } else { 0u16 };
        let mut crossx_mag = 0u32;
        let mut crossx_neg = 0u16;
        if p1_neg == p2_neg_f {
            crossx_mag = add_checked_u32(p1_mag, p2_mag);
            crossx_neg = p1_neg;
        } else if p1_mag >= p2_mag {
            crossx_mag = p1_mag - p2_mag;
            crossx_neg = p1_neg;
        } else {
            crossx_mag = p2_mag - p1_mag;
            crossx_neg = p2_neg_f;
        }

        let p3_mag = mul_checked_u32(bz_mag, cx_mag);
        let p3_neg = if bz_neg == cx_neg { 0u16 } else { 1u16 };
        let p4_mag = mul_checked_u32(bx_mag, cz_mag);
        let p4_neg = if bx_neg == cz_neg { 0u16 } else { 1u16 };
        let p4_neg_f = if p4_neg == 0u16 { 1u16 } else { 0u16 };
        let mut crossy_mag = 0u32;
        let mut crossy_neg = 0u16;
        if p3_neg == p4_neg_f {
            crossy_mag = add_checked_u32(p3_mag, p4_mag);
            crossy_neg = p3_neg;
        } else if p3_mag >= p4_mag {
            crossy_mag = p3_mag - p4_mag;
            crossy_neg = p3_neg;
        } else {
            crossy_mag = p4_mag - p3_mag;
            crossy_neg = p4_neg_f;
        }

        let p5_mag = mul_checked_u32(bx_mag, cy_mag);
        let p5_neg = if bx_neg == cy_neg { 0u16 } else { 1u16 };
        let p6_mag = mul_checked_u32(by_mag, cx_mag);
        let p6_neg = if by_neg == cx_neg { 0u16 } else { 1u16 };
        let p6_neg_f = if p6_neg == 0u16 { 1u16 } else { 0u16 };
        let mut crossz_mag = 0u32;
        let mut crossz_neg = 0u16;
        if p5_neg == p6_neg_f {
            crossz_mag = add_checked_u32(p5_mag, p6_mag);
            crossz_neg = p5_neg;
        } else if p5_mag >= p6_mag {
            crossz_mag = p5_mag - p6_mag;
            crossz_neg = p5_neg;
        } else {
            crossz_mag = p6_mag - p5_mag;
            crossz_neg = p6_neg_f;
        }

        // a . cross(b, c)
        let ax_mag = i16_mag(self.ax);
        let ax_neg = i16_neg(self.ax);
        let ay_mag = i16_mag(self.ay);
        let ay_neg = i16_neg(self.ay);
        let az_mag = i16_mag(self.az);
        let az_neg = i16_neg(self.az);

        let t1_mag = mul_checked_u32(ax_mag, crossx_mag);
        let t1_neg = if ax_neg == crossx_neg { 0u16 } else { 1u16 };
        let t2_mag = mul_checked_u32(ay_mag, crossy_mag);
        let t2_neg = if ay_neg == crossy_neg { 0u16 } else { 1u16 };
        let t3_mag = mul_checked_u32(az_mag, crossz_mag);
        let t3_neg = if az_neg == crossz_neg { 0u16 } else { 1u16 };

        let mut s1_mag = 0u32;
        let mut s1_neg = 0u16;
        if t1_neg == t2_neg {
            s1_mag = add_checked_u32(t1_mag, t2_mag);
            s1_neg = t1_neg;
        } else if t1_mag >= t2_mag {
            s1_mag = t1_mag - t2_mag;
            s1_neg = t1_neg;
        } else {
            s1_mag = t2_mag - t1_mag;
            s1_neg = t2_neg;
        }

        let mut s2_mag = 0u32;
        let mut s2_neg = 0u16;
        if s1_neg == t3_neg {
            s2_mag = add_checked_u32(s1_mag, t3_mag);
            s2_neg = s1_neg;
        } else if s1_mag >= t3_mag {
            s2_mag = s1_mag - t3_mag;
            s2_neg = s1_neg;
        } else {
            s2_mag = t3_mag - s1_mag;
            s2_neg = t3_neg;
        }

        self.result_mag = s2_mag;
        self.result_neg = s2_neg;
        1u16
    }
}
