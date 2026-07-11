//! True (non-squared) Euclidean distance between two grid points with signed (i16) coordinates: isqrt(dx*dx + dy*dy), each coordinate difference computed via an excess-32768 shift feeding the shared iabs_diff kernel (the chebyshev_i16/manhattan_i16 technique) into euclid_dist's own add_checked_u32 + inline branch-free isqrt loop -- the signed sibling euclid_dist lacks, since its u16-only fields can't take an origin-centered coordinate at all; distinct from euclid_sq_i16 by rooting the sum rather than leaving it squared.
//! tags: grid, distance, euclidean, sqrt, root, spatial, magnitude, wide, u32, signed, i16, checked, escalate
//! entry: PtsSigned::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if dx*dx + dy*dy exceeds u32::MAX (both axes near-maximally separated at once)
struct PtsSigned { x1: i16, y1: i16, x2: i16, y2: i16, dist: u16 }
impl PtsSigned {
    fn run(&mut self) -> u16 {
        let sx1 = (self.x1 as u16).wrapping_add(32768u16);
        let sx2 = (self.x2 as u16).wrapping_add(32768u16);
        let sy1 = (self.y1 as u16).wrapping_add(32768u16);
        let sy2 = (self.y2 as u16).wrapping_add(32768u16);
        let dx = iabs_diff(sx1, sx2);
        let dy = iabs_diff(sy1, sy2);
        let sum = add_checked_u32((dx as u32) * (dx as u32), (dy as u32) * (dy as u32));

        // Branch-free bitwise integer square root of sum (the same loop euclid_dist/isqrt_u32/q_sqrt/cosine_score_approx run inline).
        let mut val = sum;
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
