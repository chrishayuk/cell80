//! Squared Euclidean distance between two 3D points -- the missing 3D sibling of euclid_sq, which stays squared for the same reason euclid_sq does (no square root in the dialect). Each signed coordinate difference is computed via an excess-32768 shift (mapping i16's range onto u16 losslessly) feeding the shared iabs_diff kernel, so no i16 subtraction ever risks overflowing i16's own range.
//! tags: geometry, distance, 3d, euclidean, squared, vector, point, wide, u32, checked, escalate
//! entry: GeomDistance3d::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if the summed squared distance would exceed u32::MAX
struct GeomDistance3d { ax: i16, ay: i16, az: i16, bx: i16, by: i16, bz: i16, result: u32 }
impl GeomDistance3d {
    fn run(&mut self) -> u16 {
        let sax = (self.ax as u16).wrapping_add(32768u16);
        let sbx = (self.bx as u16).wrapping_add(32768u16);
        let say = (self.ay as u16).wrapping_add(32768u16);
        let sby = (self.by as u16).wrapping_add(32768u16);
        let saz = (self.az as u16).wrapping_add(32768u16);
        let sbz = (self.bz as u16).wrapping_add(32768u16);
        let dx = iabs_diff(sax, sbx);
        let dy = iabs_diff(say, sby);
        let dz = iabs_diff(saz, sbz);
        let dx_sq = dx as u32 * dx as u32;
        let dy_sq = dy as u32 * dy as u32;
        let dz_sq = dz as u32 * dz as u32;
        let sum1 = add_checked_u32(dx_sq, dy_sq);
        let total = add_checked_u32(sum1, dz_sq);
        self.result = total;
        1u16
    }
}
