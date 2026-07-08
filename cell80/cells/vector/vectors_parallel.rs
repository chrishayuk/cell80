//! Check whether two 3D vectors are parallel (or anti-parallel) -- one is a scalar multiple of the other. Computed via three pairwise-product equality checks (same magnitude, same sign, or either magnitude zero) rather than a signed subtract, so no sign-combining step is needed at all.
//! tags: vector, parallel, anti-parallel, scalar-multiple, geometry, 3d, predicate, collinear, direction
//! entry: VectorsParallel::run
fn i16_mag(v: i16) -> u32 {
    if v < 0i16 { (0u16.wrapping_sub(v as u16)) as u32 } else { v as u16 as u32 }
}
fn i16_neg(v: i16) -> u16 {
    if v < 0i16 { 1u16 } else { 0u16 }
}
struct VectorsParallel { ax: i16, ay: i16, az: i16, bx: i16, by: i16, bz: i16, result: u16 }
impl VectorsParallel {
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

        let p1_mag = ay_mag * bz_mag;
        let p1_neg = if ay_neg == bz_neg { 0u16 } else { 1u16 };
        let p2_mag = az_mag * by_mag;
        let p2_neg = if az_neg == by_neg { 0u16 } else { 1u16 };
        let mut eq1 = 0u16;
        if p1_mag == p2_mag && (p1_neg == p2_neg || p1_mag == 0u32) { eq1 = 1u16; }

        let p3_mag = az_mag * bx_mag;
        let p3_neg = if az_neg == bx_neg { 0u16 } else { 1u16 };
        let p4_mag = ax_mag * bz_mag;
        let p4_neg = if ax_neg == bz_neg { 0u16 } else { 1u16 };
        let mut eq2 = 0u16;
        if p3_mag == p4_mag && (p3_neg == p4_neg || p3_mag == 0u32) { eq2 = 1u16; }

        let p5_mag = ax_mag * by_mag;
        let p5_neg = if ax_neg == by_neg { 0u16 } else { 1u16 };
        let p6_mag = ay_mag * bx_mag;
        let p6_neg = if ay_neg == bx_neg { 0u16 } else { 1u16 };
        let mut eq3 = 0u16;
        if p5_mag == p6_mag && (p5_neg == p6_neg || p5_mag == 0u32) { eq3 = 1u16; }

        let mut result = 0u16;
        if eq1 == 1u16 && eq2 == 1u16 && eq3 == 1u16 { result = 1u16; }
        self.result = result;
        1u16
    }
}
