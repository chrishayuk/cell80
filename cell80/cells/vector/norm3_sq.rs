//! Squared magnitude of a signed 3D vector (x, y, z): x*x + y*y + z*z, widened to a u32 field -- the signed-input, 3D sibling of norm2_sq (which is 2D, unsigned, and stays in u16); needs a state cell purely because u32 can't be a free-fn return type even with only 3 inputs. Each component's square is always non-negative, so this tracks magnitude only via i16_mag and never needs the sign-combining step cross_product/triple_scalar_product require for their differences of products.
//! tags: vector, norm, magnitude, squared, length, 3d, signed, wide, u32, checked
//! entry: Norm3Sq::run
//! limits: escalates (halt 0xFF05, needs_wider_math) on the (unreachable in practice for i16 inputs) intermediate overflow the shared mul_checked_u32/add_checked_u32 kernels guard
fn i16_mag(v: i16) -> u32 {
    if v < 0i16 { (0u16.wrapping_sub(v as u16)) as u32 } else { v as u16 as u32 }
}
struct Norm3Sq { x: i16, y: i16, z: i16, mag_sq: u32 }
impl Norm3Sq {
    fn run(&mut self) -> u16 {
        let x_mag = i16_mag(self.x);
        let y_mag = i16_mag(self.y);
        let z_mag = i16_mag(self.z);

        let x_sq = mul_checked_u32(x_mag, x_mag);
        let y_sq = mul_checked_u32(y_mag, y_mag);
        let z_sq = mul_checked_u32(z_mag, z_mag);

        let sum1 = add_checked_u32(x_sq, y_sq);
        let sum2 = add_checked_u32(sum1, z_sq);

        self.mag_sq = sum2;
        1u16
    }
}
