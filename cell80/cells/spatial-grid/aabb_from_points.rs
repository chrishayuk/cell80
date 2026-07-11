//! Normalizes two arbitrary, unordered corner points (x1,y1) and (x2,y2) into a well-formed AABB (x,y,w,h) -- the natural input shape for a drag-select rectangle, unlike every other aabb_* cell here which takes two already-formed boxes.
//! tags: grid, aabb, rect, corners, points, normalize, bounding, spatial, drag-select
//! entry: AabbFromPoints::run
struct AabbFromPoints { x1: u16, y1: u16, x2: u16, y2: u16, x: u16, y: u16, w: u16, h: u16 }
impl AabbFromPoints {
    fn run(&mut self) -> u16 {
        self.x = imin(self.x1, self.x2);
        self.y = imin(self.y1, self.y2);
        self.w = iabs_diff(self.x1, self.x2);
        self.h = iabs_diff(self.y1, self.y2);
        self.x
    }
}
