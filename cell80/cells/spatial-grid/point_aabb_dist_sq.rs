//! Exact squared distance from a point (px,py) to the nearest point on/in an axis-aligned box (rx,ry,rw,rh) -- 0 if the point lies inside -- via clamping the point into the box's own range on each axis then squaring the residual offset; the AABB counterpart to point_in_circle's point-to-circle test and geometry's point_segment_dist_sq, since point_line_dist_sq (this library's only other distance-to-shape primitive) is point-to-infinite-line, a genuinely different shape with no clamping step at all.
//! tags: grid, aabb, rect, distance, point, squared, clamp, nearest, spatial, bounds, no-sqrt, quadtree, pruning
//! entry: PointAabbDistSq::run
struct PointAabbDistSq { px: u16, py: u16, rx: u16, ry: u16, rw: u16, rh: u16, dist_sq: u32 }
impl PointAabbDistSq {
    fn run(&mut self) -> u16 {
        let hi_x = self.rx + self.rw;
        let hi_y = self.ry + self.rh;
        let cx = if self.px > hi_x { hi_x } else if self.px < self.rx { self.rx } else { self.px };
        let cy = if self.py > hi_y { hi_y } else if self.py < self.ry { self.ry } else { self.py };
        let dx = iabs_diff(self.px, cx);
        let dy = iabs_diff(self.py, cy);
        self.dist_sq = dx as u32 * dx as u32 + dy as u32 * dy as u32;
        if (self.dist_sq >> 16u32) as u16 != 0u16 { 65535u16 } else { self.dist_sq as u16 }
    }
}
