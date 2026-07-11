//! Total true (non-squared) Euclidean path distance across three consecutive points (x1,y1)->(x2,y2)->(x3,y3): euclid_dist(p1,p2) + euclid_dist(p2,p3) -- the real-world-distance sibling of manhattan_path3, one metric over (rooted per-segment, not summed-then-never-rooted). Each segment is computed via euclid_dist's own excess-shift/add_checked_u32/inline-isqrt chain, inlined twice since isqrt_u32 is itself a state cell and cannot be called as a subroutine across a call boundary.
//! tags: grid, distance, euclidean, sqrt, root, spatial, path, waypoint, three-point, wide, u32
//! entry: Path3::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if either segment's dx*dx + dy*dy sum exceeds u32::MAX
struct Path3 { x1: u16, y1: u16, x2: u16, y2: u16, x3: u16, y3: u16, dist: u32 }
impl Path3 {
    fn run(&mut self) -> u16 {
        // Segment 1: (x1,y1) -> (x2,y2), euclid_dist's own inline chain.
        let dx1 = iabs_diff(self.x1, self.x2);
        let dy1 = iabs_diff(self.y1, self.y2);
        let sum1 = add_checked_u32((dx1 as u32) * (dx1 as u32), (dy1 as u32) * (dy1 as u32));
        let mut val1 = sum1;
        let mut res1 = 0u32;
        let mut bit1 = 1u32 << 30u32;
        while bit1 > val1 { bit1 = bit1 >> 2u32; }
        while bit1 != 0u32 {
            if val1 >= res1 + bit1 {
                val1 = val1 - (res1 + bit1);
                res1 = (res1 >> 1u32) + bit1;
            } else {
                res1 = res1 >> 1u32;
            }
            bit1 = bit1 >> 2u32;
        }

        // Segment 2: (x2,y2) -> (x3,y3), the same chain inlined a second time.
        let dx2 = iabs_diff(self.x2, self.x3);
        let dy2 = iabs_diff(self.y2, self.y3);
        let sum2 = add_checked_u32((dx2 as u32) * (dx2 as u32), (dy2 as u32) * (dy2 as u32));
        let mut val2 = sum2;
        let mut res2 = 0u32;
        let mut bit2 = 1u32 << 30u32;
        while bit2 > val2 { bit2 = bit2 >> 2u32; }
        while bit2 != 0u32 {
            if val2 >= res2 + bit2 {
                val2 = val2 - (res2 + bit2);
                res2 = (res2 >> 1u32) + bit2;
            } else {
                res2 = res2 >> 1u32;
            }
            bit2 = bit2 >> 2u32;
        }

        self.dist = add_checked_u32(res1, res2);
        if (self.dist >> 16u32) as u16 != 0u16 { 65535u16 } else { self.dist as u16 }
    }
}
