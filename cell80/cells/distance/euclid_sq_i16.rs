//! Squared Euclidean distance between two grid points with signed (i16) coordinates: dx*dx + dy*dy (no sqrt) into a wide u32 dist field, each coordinate difference computed via an excess-32768 shift feeding the shared iabs_diff kernel (the manhattan_i16/chebyshev_i16 technique) -- the 2D signed sibling euclid_sq lacks, since its u16-only fields can't take an origin-centered coordinate at all (distinct from geom_distance_3d, which is the same idea but for a third axis). The two squared terms are combined via the shared add_checked_u32 kernel (the geom_distance_3d technique) so a maximally-separated pair escalates instead of silently wrapping past u32::MAX.
//! tags: grid, distance, spatial, score, navigation, signed, i16, euclidean, squared, wide, u32, checked, escalate
//! entry: PtsSigned::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if dx*dx + dy*dy exceeds u32::MAX (both axes near-maximally separated at once)
struct PtsSigned { x1: i16, y1: i16, x2: i16, y2: i16, dist: u32 }
impl PtsSigned {
    fn run(&mut self) -> u16 {
        let sx1 = (self.x1 as u16).wrapping_add(32768u16);
        let sx2 = (self.x2 as u16).wrapping_add(32768u16);
        let sy1 = (self.y1 as u16).wrapping_add(32768u16);
        let sy2 = (self.y2 as u16).wrapping_add(32768u16);
        let dx = iabs_diff(sx1, sx2);
        let dy = iabs_diff(sy1, sy2);
        let dx_sq = dx as u32 * dx as u32;
        let dy_sq = dy as u32 * dy as u32;
        self.dist = add_checked_u32(dx_sq, dy_sq);
        if (self.dist >> 16u32) as u16 != 0u16 { 65535u16 } else { self.dist as u16 }
    }
}
