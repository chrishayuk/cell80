//! Returns 1 if two axis-aligned bounding boxes (x1,y1,w1,h1) and (x2,y2,w2,h2) overlap (edge-touching doesn't count), else 0.
//! tags: grid, aabb, rect, intersect, overlap, collision, spatial
//! entry: AabbIntersect::run
struct AabbIntersect { x1: u16, y1: u16, w1: u16, h1: u16, x2: u16, y2: u16, w2: u16, h2: u16, overlap: u16 }
impl AabbIntersect {
    fn run(&mut self) -> u16 {
        let overlap = (self.x1 < self.x2 + self.w2)
            && (self.x2 < self.x1 + self.w1)
            && (self.y1 < self.y2 + self.h2)
            && (self.y2 < self.y1 + self.h1);
        self.overlap = overlap as u16;
        self.overlap
    }
}
