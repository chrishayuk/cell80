//! Euclidean length (not squared) of a signed 3D vector (x, y, z): floor(sqrt(x*x + y*y + z*z)) -- the long-blocked square-root sibling of norm3_sq (which stops at the squared magnitude only because u32 can't be a free-fn return type to hand a sqrt result back through), closed now that isqrt_u32 supplies a wide integer sqrt. Reuses norm3_sq's exact i16_mag/mul_checked_u32/add_checked_u32 chain to build the squared magnitude as an internal u32, then runs isqrt_u32's branch-free bitwise loop on it inline -- the same technique that unblocked cosine_score_approx.
//! tags: vector, length, magnitude, norm, 3d, signed, sqrt, euclidean, wide, u32
//! entry: Vec3Length::run
//! limits: escalates (halt 0xFF05, needs_wider_math) on the (unreachable in practice for i16 inputs) intermediate overflow the shared mul_checked_u32/add_checked_u32 kernels guard
fn i16_mag(v: i16) -> u32 {
    if v < 0i16 { (0u16.wrapping_sub(v as u16)) as u32 } else { v as u16 as u32 }
}
struct Vec3Length { x: i16, y: i16, z: i16, len: u16 }
impl Vec3Length {
    fn run(&mut self) -> u16 {
        let x_mag = i16_mag(self.x);
        let y_mag = i16_mag(self.y);
        let z_mag = i16_mag(self.z);

        let x_sq = mul_checked_u32(x_mag, x_mag);
        let y_sq = mul_checked_u32(y_mag, y_mag);
        let z_sq = mul_checked_u32(z_mag, z_mag);

        let sum1 = add_checked_u32(x_sq, y_sq);
        let mag_sq = add_checked_u32(sum1, z_sq);

        // Branch-free bitwise integer square root of mag_sq (the same loop q_sqrt/isqrt_u32 run).
        let mut val = mag_sq;
        let mut res = 0u32;
        let mut bit = 1u32 << 30u32;
        while bit > val { bit = bit >> 2u32; }
        while bit != 0u32 {
            if val >= res + bit {
                val = val - (res + bit);
                res = (res >> 1u32) + bit;
            } else {
                res = res >> 1u32;
            }
            bit = bit >> 2u32;
        }

        let len = res as u16;
        self.len = len;
        len
    }
}
