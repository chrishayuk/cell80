//! Center point (cx,cy) of a single AABB (x,y,w,h): cx=x+w/2, cy=y+h/2 -- unlike aabb_union/aabb_intersection/aabb_contains, which all relate two boxes to each other, this derives a representative point from one box (quad-tree split points, spatial hashing keys).
//! tags: grid, aabb, rect, center, centroid, midpoint, spatial, bbox
//! entry: AabbCenter::run
struct AabbCenter { x: u16, y: u16, w: u16, h: u16, cx: u16, cy: u16 }
impl AabbCenter {
    fn run(&mut self) -> u16 {
        let cx = self.x + self.w / 2u16;
        let cy = self.y + self.h / 2u16;
        self.cx = cx;
        self.cy = cy;
        self.cx
    }
}
