//! Triple vector product a x (b x c) of three 3D vectors, via the BAC-CAB identity a x (b x c) = b*(a.c) - c*(a.b) -- pure dot-products and scalar multiplies, never an actual cross-product computation. Each stage (the two dot products, the two vector scalings, the final vector subtract) is tracked as a (magnitude, sign) pair, the same discipline cross_product/triple_scalar_product use. Genuinely narrower-range than those two: scaling a vector component by a dot product can reach i16's product-of-three-factors territory, so this escalates well before either input vector's own magnitude would suggest trouble.
//! tags: vector, triple-product, vector-triple-product, bac-cab, geometry, 3d, wide, u32, checked, escalate
//! entry: TripleVectorProduct::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if an intermediate dot product, scaling, or combining step overflows u32 -- the scaling step (a dot product times a vector component) can overflow for inputs well within i16's own range, since it is effectively a product of three i16-scale factors
fn i16_mag(v: i16) -> u32 {
    if v < 0i16 { (0u16.wrapping_sub(v as u16)) as u32 } else { v as u16 as u32 }
}
fn i16_neg(v: i16) -> u16 {
    if v < 0i16 { 1u16 } else { 0u16 }
}
struct TripleVectorProduct {
    ax: i16, ay: i16, az: i16,
    bx: i16, by: i16, bz: i16,
    cx: i16, cy: i16, cz: i16,
    rx_mag: u32, rx_neg: u16, ry_mag: u32, ry_neg: u16, rz_mag: u32, rz_neg: u16
}
impl TripleVectorProduct {
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
        let cx_mag = i16_mag(self.cx);
        let cx_neg = i16_neg(self.cx);
        let cy_mag = i16_mag(self.cy);
        let cy_neg = i16_neg(self.cy);
        let cz_mag = i16_mag(self.cz);
        let cz_neg = i16_neg(self.cz);

        // dot(a, c)
        let p1_mag = mul_checked_u32(ax_mag, cx_mag);
        let p1_neg = if ax_neg == cx_neg { 0u16 } else { 1u16 };
        let p2_mag = mul_checked_u32(ay_mag, cy_mag);
        let p2_neg = if ay_neg == cy_neg { 0u16 } else { 1u16 };
        let mut s1_mag = 0u32;
        let mut s1_neg = 0u16;
        if p1_neg == p2_neg {
            s1_mag = add_checked_u32(p1_mag, p2_mag);
            s1_neg = p1_neg;
        } else if p1_mag >= p2_mag {
            s1_mag = p1_mag - p2_mag;
            s1_neg = p1_neg;
        } else {
            s1_mag = p2_mag - p1_mag;
            s1_neg = p2_neg;
        }
        let p3_mag = mul_checked_u32(az_mag, cz_mag);
        let p3_neg = if az_neg == cz_neg { 0u16 } else { 1u16 };
        let mut ac_mag = 0u32;
        let mut ac_neg = 0u16;
        if s1_neg == p3_neg {
            ac_mag = add_checked_u32(s1_mag, p3_mag);
            ac_neg = s1_neg;
        } else if s1_mag >= p3_mag {
            ac_mag = s1_mag - p3_mag;
            ac_neg = s1_neg;
        } else {
            ac_mag = p3_mag - s1_mag;
            ac_neg = p3_neg;
        }

        // dot(a, b)
        let p4_mag = mul_checked_u32(ax_mag, bx_mag);
        let p4_neg = if ax_neg == bx_neg { 0u16 } else { 1u16 };
        let p5_mag = mul_checked_u32(ay_mag, by_mag);
        let p5_neg = if ay_neg == by_neg { 0u16 } else { 1u16 };
        let mut s2_mag = 0u32;
        let mut s2_neg = 0u16;
        if p4_neg == p5_neg {
            s2_mag = add_checked_u32(p4_mag, p5_mag);
            s2_neg = p4_neg;
        } else if p4_mag >= p5_mag {
            s2_mag = p4_mag - p5_mag;
            s2_neg = p4_neg;
        } else {
            s2_mag = p5_mag - p4_mag;
            s2_neg = p5_neg;
        }
        let p6_mag = mul_checked_u32(az_mag, bz_mag);
        let p6_neg = if az_neg == bz_neg { 0u16 } else { 1u16 };
        let mut ab_mag = 0u32;
        let mut ab_neg = 0u16;
        if s2_neg == p6_neg {
            ab_mag = add_checked_u32(s2_mag, p6_mag);
            ab_neg = s2_neg;
        } else if s2_mag >= p6_mag {
            ab_mag = s2_mag - p6_mag;
            ab_neg = s2_neg;
        } else {
            ab_mag = p6_mag - s2_mag;
            ab_neg = p6_neg;
        }

        // result = b*(a.c) - c*(a.b), component by component
        let bx_ac_mag = mul_checked_u32(bx_mag, ac_mag);
        let bx_ac_neg = if bx_neg == ac_neg { 0u16 } else { 1u16 };
        let cx_ab_mag = mul_checked_u32(cx_mag, ab_mag);
        let cx_ab_neg = if cx_neg == ab_neg { 0u16 } else { 1u16 };
        let cx_ab_neg_f = if cx_ab_neg == 0u16 { 1u16 } else { 0u16 };
        let mut rx_mag = 0u32;
        let mut rx_neg = 0u16;
        if bx_ac_neg == cx_ab_neg_f {
            rx_mag = add_checked_u32(bx_ac_mag, cx_ab_mag);
            rx_neg = bx_ac_neg;
        } else if bx_ac_mag >= cx_ab_mag {
            rx_mag = bx_ac_mag - cx_ab_mag;
            rx_neg = bx_ac_neg;
        } else {
            rx_mag = cx_ab_mag - bx_ac_mag;
            rx_neg = cx_ab_neg_f;
        }

        let by_ac_mag = mul_checked_u32(by_mag, ac_mag);
        let by_ac_neg = if by_neg == ac_neg { 0u16 } else { 1u16 };
        let cy_ab_mag = mul_checked_u32(cy_mag, ab_mag);
        let cy_ab_neg = if cy_neg == ab_neg { 0u16 } else { 1u16 };
        let cy_ab_neg_f = if cy_ab_neg == 0u16 { 1u16 } else { 0u16 };
        let mut ry_mag = 0u32;
        let mut ry_neg = 0u16;
        if by_ac_neg == cy_ab_neg_f {
            ry_mag = add_checked_u32(by_ac_mag, cy_ab_mag);
            ry_neg = by_ac_neg;
        } else if by_ac_mag >= cy_ab_mag {
            ry_mag = by_ac_mag - cy_ab_mag;
            ry_neg = by_ac_neg;
        } else {
            ry_mag = cy_ab_mag - by_ac_mag;
            ry_neg = cy_ab_neg_f;
        }

        let bz_ac_mag = mul_checked_u32(bz_mag, ac_mag);
        let bz_ac_neg = if bz_neg == ac_neg { 0u16 } else { 1u16 };
        let cz_ab_mag = mul_checked_u32(cz_mag, ab_mag);
        let cz_ab_neg = if cz_neg == ab_neg { 0u16 } else { 1u16 };
        let cz_ab_neg_f = if cz_ab_neg == 0u16 { 1u16 } else { 0u16 };
        let mut rz_mag = 0u32;
        let mut rz_neg = 0u16;
        if bz_ac_neg == cz_ab_neg_f {
            rz_mag = add_checked_u32(bz_ac_mag, cz_ab_mag);
            rz_neg = bz_ac_neg;
        } else if bz_ac_mag >= cz_ab_mag {
            rz_mag = bz_ac_mag - cz_ab_mag;
            rz_neg = bz_ac_neg;
        } else {
            rz_mag = cz_ab_mag - bz_ac_mag;
            rz_neg = cz_ab_neg_f;
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
