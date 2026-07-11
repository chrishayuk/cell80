//! True (non-squared) Euclidean distance between two 3D points (ax,ay,az)-(bx,by,bz): isqrt(dx*dx + dy*dy + dz*dz) -- the isqrt-closed sibling of geom_distance_3d, whose own docstring cites "no square root in the dialect" as the reason it stays squared, a blocker isqrt_u32 already closed for the 2D two-point case (euclid_dist) and the one-vector case (vec3_length) but never for two-point 3D distance until now; distinct from vec3_length, which takes an already-relative vector rather than two points needing their own difference computed. Reuses geom_distance_3d's exact excess-32768-shift + iabs_diff + add_checked_u32 chain to build the squared sum internally, then runs the same branch-free bitwise isqrt loop euclid_dist/vec3_length/triangle_area_x4_approx already inline.
//! tags: geometry, distance, 3d, euclidean, sqrt, root, vector, point, wide, u32, checked, escalate
//! entry: GeomDistance3dExact::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if the summed squared distance would exceed u32::MAX
struct GeomDistance3dExact { ax: i16, ay: i16, az: i16, bx: i16, by: i16, bz: i16, dist: u16 }
impl GeomDistance3dExact {
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

        // Branch-free bitwise integer square root of total (the same loop euclid_dist/vec3_length/isqrt_u32 run inline).
        let mut val = total;
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

        self.dist = res as u16;
        self.dist
    }
}
