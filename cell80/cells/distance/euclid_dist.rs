//! True (non-squared) Euclidean distance between two grid points: isqrt(dx*dx + dy*dy) -- the sqrt-closed sibling of euclid_sq, whose own docstring gives "(no sqrt)" as its reason for staying squared, a blocker isqrt_u32's wide integer sqrt now removes the same way it unblocked cosine_score_approx. dx*dx and dy*dy are combined via the shared add_checked_u32 kernel so an extreme dx/dy pair escalates instead of silently wrapping, then reduced with the same branch-free bitwise integer-sqrt loop isqrt_u32/q_sqrt/cosine_score_approx run inline.
//! tags: grid, distance, euclidean, sqrt, root, spatial, magnitude, wide, u32
//! entry: Pts::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if dx*dx + dy*dy exceeds u32::MAX (both axes near-maximally separated at once)
struct Pts { x1: u16, y1: u16, x2: u16, y2: u16, dist: u16 }
impl Pts {
    fn run(&mut self) -> u16 {
        let dx = iabs_diff(self.x1, self.x2);
        let dy = iabs_diff(self.y1, self.y2);
        let sum = add_checked_u32((dx as u32) * (dx as u32), (dy as u32) * (dy as u32));

        // Branch-free bitwise integer square root of sum (the same loop q_sqrt/isqrt_u32/cosine_score_approx run inline).
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
