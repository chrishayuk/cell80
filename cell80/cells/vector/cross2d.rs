//! Signed scalar 2D cross product of two 2D vectors (ax,ay),(bx,by): ax*by - ay*bx, returned as an exact (magnitude, sign) pair -- orientation2d computes this same quantity internally (from 3 points rather than 2 vectors) but discards the magnitude down to a -1/0/1 turn sign; cross_product already exposes the full signed magnitude for its 3D result, this is the missing 2D scalar analogue, using that same combining-subtract technique for its single component but simplified since ax/ay/bx/by are plain u16 magnitudes (no i16 sign-tracking needed on the inputs themselves).
//! tags: vector, cross-product, 2d, geometry, signed, wide, u32, orientation, scalar
//! entry: Cross2d::run
struct Cross2d { ax: u16, ay: u16, bx: u16, by: u16, cross_mag: u32, cross_neg: u16 }
impl Cross2d {
    fn run(&mut self) -> u16 {
        let p1_mag = (self.ax as u32) * (self.by as u32);
        let p2_mag = (self.ay as u32) * (self.bx as u32);

        let mut cross_mag = 0u32;
        let mut cross_neg = 0u16;
        if p1_mag >= p2_mag {
            cross_mag = p1_mag - p2_mag;
            cross_neg = 0u16;
        } else {
            cross_mag = p2_mag - p1_mag;
            cross_neg = 1u16;
        }

        self.cross_mag = cross_mag;
        self.cross_neg = cross_neg;
        1u16
    }
}
