//! Check whether two signed 3D vectors are orthogonal (perpendicular) -- dot3(a,b) == 0 -- reusing dot3's exact sign-magnitude product/sum chain internally and testing the final magnitude, distinct from vectors_parallel's cross-product-zero relationship (most pairs are neither parallel nor perpendicular).
//! tags: vector, orthogonal, perpendicular, dot-product, signed, 3d, predicate, geometry
//! entry: VectorsOrthogonal::run
fn i16_mag(v: i16) -> u32 {
    if v < 0i16 { (0u16.wrapping_sub(v as u16)) as u32 } else { v as u16 as u32 }
}
fn i16_neg(v: i16) -> u16 {
    if v < 0i16 { 1u16 } else { 0u16 }
}
struct VectorsOrthogonal { ax: i16, ay: i16, az: i16, bx: i16, by: i16, bz: i16, result: u16 }
impl VectorsOrthogonal {
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

        // p1 = ax*bx, p2 = ay*by, p3 = az*bz -- each a signed (magnitude, sign) product.
        let p1_mag = mul_checked_u32(ax_mag, bx_mag);
        let mut p1_neg = if ax_neg == bx_neg { 0u16 } else { 1u16 };
        if p1_mag == 0u32 { p1_neg = 0u16; }
        let p2_mag = mul_checked_u32(ay_mag, by_mag);
        let mut p2_neg = if ay_neg == by_neg { 0u16 } else { 1u16 };
        if p2_mag == 0u32 { p2_neg = 0u16; }
        let p3_mag = mul_checked_u32(az_mag, bz_mag);
        let mut p3_neg = if az_neg == bz_neg { 0u16 } else { 1u16 };
        if p3_mag == 0u32 { p3_neg = 0u16; }

        // s1 = p1 + p2: same sign adds magnitudes, opposite sign subtracts the smaller
        // magnitude from the larger and takes the larger operand's sign.
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
        if s1_mag == 0u32 { s1_neg = 0u16; }

        // s2 = s1 + p3, the same way -- this is the full dot3 magnitude.
        let mut s2_mag = 0u32;
        let mut s2_neg = 0u16;
        if s1_neg == p3_neg {
            s2_mag = add_checked_u32(s1_mag, p3_mag);
            s2_neg = s1_neg;
        } else if s1_mag >= p3_mag {
            s2_mag = s1_mag - p3_mag;
            s2_neg = s1_neg;
        } else {
            s2_mag = p3_mag - s1_mag;
            s2_neg = p3_neg;
        }
        if s2_mag == 0u32 { s2_neg = 0u16; }

        let mut result = 0u16;
        if s2_mag == 0u32 { result = 1u16; }
        self.result = result;
        1u16
    }
}
