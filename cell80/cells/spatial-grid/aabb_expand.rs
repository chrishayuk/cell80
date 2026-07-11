//! Inflates AABB (x,y,w,h) outward by a uniform margin on all four sides -- clamped at the low edge (x/y saturate at 0) and the high edge (w/h saturate at u16::MAX) -- unlike aabb_union/aabb_intersection/aabb_contains, which all relate two boxes to each other, this transforms a single box by a scalar (broad-phase collision padding, hitbox slop).
//! tags: grid, aabb, rect, expand, inflate, margin, padding, spatial, bbox
//! entry: AabbExpand::run
struct AabbExpand { x: u16, y: u16, w: u16, h: u16, margin: u16, nx: u16, ny: u16, nw: u16, nh: u16 }
impl AabbExpand {
    fn run(&mut self) -> u16 {
        let nx = self.x.saturating_sub(self.margin);
        let ny = self.y.saturating_sub(self.margin);
        let nw = self.w.saturating_add(self.margin.saturating_mul(2u16));
        let nh = self.h.saturating_add(self.margin.saturating_mul(2u16));
        self.nx = nx;
        self.ny = ny;
        self.nw = nw;
        self.nh = nh;
        self.nx
    }
}
