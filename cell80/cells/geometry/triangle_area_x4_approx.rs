//! Floor(4*Area) of a triangle with integer sides (a, b, c): computes 16*Area^2 exactly the way heron_16a2 does (Heron's formula rearranged to avoid a square root), then extracts an actual usable magnitude via the same branch-free bitwise integer-sqrt loop isqrt_u32 runs, since isqrt(16*Area^2) = floor(4*Area) -- heron_16a2's own doc comment says it stays squared only because no wide sqrt existed when it was authored; this is the first geometry cell to surface a real area magnitude instead.
//! tags: geometry, triangle, area, heron, sqrt, isqrt, wide, u32, magnitude, floor
//! entry: TriangleAreaX4Approx::run
//! limits: escalates (halt 0xFF06, out_of_domain) if a, b, c do not form a valid (non-degenerate) triangle; escalates (halt 0xFF05, needs_wider_math) if either factor-pair product overflows u32
struct TriangleAreaX4Approx { a: u16, b: u16, c: u16, area_x4: u32 }
impl TriangleAreaX4Approx {
    fn run(&mut self) -> u16 {
        let aw = self.a as u32;
        let bw = self.b as u32;
        let cw = self.c as u32;
        if aw + bw <= cw || bw + cw <= aw || aw + cw <= bw { halt(0xFF06u16); }
        let s1 = aw + bw + cw;
        let s2 = bw + cw - aw;
        let s3 = aw + cw - bw;
        let s4 = aw + bw - cw;
        let p1 = mul_checked_u32(s1, s2);
        let p2 = mul_checked_u32(s3, s4);
        let sixteen_a2 = mul_checked_u32(p1, p2);

        // Branch-free bitwise integer square root of sixteen_a2 (the same loop isqrt_u32 runs).
        let mut val = sixteen_a2;
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

        self.area_x4 = res;
        res as u16
    }
}
