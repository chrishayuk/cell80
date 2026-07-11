//! Approximate cosine similarity of two 2D vectors as a Q8.8 score in [0, 256] (1.0 = parallel, 0 = perpendicular): dot / sqrt(norm_a * norm_b), the long-blocked vector-pack candidate closed by isqrt_u32's wide integer sqrt -- norm_a and norm_b are each at most u16::MAX, so their u32 product (up to 65535*65535) always fits u32 with room to spare, sidestepping the sqrt-of-a-product overflow this cell was parked behind. Same modest-magnitude domain as dot2/norm2_sq (plain u16 arithmetic, silently wraps past that domain, not a new limitation).
//! tags: vector, cosine, similarity, score, angle, normalized, dot-product, fixed-point, q8.8, sqrt
//! entry: CosineScoreApprox::run
//! limits: returns 0 for a zero-magnitude input vector (undefined cosine, the safe_div convention); inherits dot2/norm2_sq's own u16-wraparound domain limit for large components
struct CosineScoreApprox { ax: u16, ay: u16, bx: u16, by: u16, score: u16 }
impl CosineScoreApprox {
    fn run(&mut self) -> u16 {
        let dot = self.ax * self.bx + self.ay * self.by;
        let norm_a = self.ax * self.ax + self.ay * self.ay;
        let norm_b = self.bx * self.bx + self.by * self.by;
        let prod = (norm_a as u32) * (norm_b as u32);

        // Branch-free bitwise integer square root of prod (the same loop q_sqrt/isqrt_u32 run).
        let mut val = prod;
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

        let score = if res == 0u32 { 0u16 } else { (((dot as u32) << 8u32) / res) as u16 };
        self.score = score;
        score
    }
}
