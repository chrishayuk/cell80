//! Returns 1 if AABB (x2,y2,w2,h2) is fully contained within AABB (x1,y1,w1,h1) — all four edges inside — else 0.
//! tags: grid, aabb, rect, contains, containment, spatial, bounds
//! entry: AabbContains::run
struct AabbContains { x1: u16, y1: u16, w1: u16, h1: u16, x2: u16, y2: u16, w2: u16, h2: u16, contains: u16 }
impl AabbContains {
    fn run(&mut self) -> u16 {
        let contains = (self.x2 >= self.x1)
            && (self.y2 >= self.y1)
            && (self.x2 + self.w2 <= self.x1 + self.w1)
            && (self.y2 + self.h2 <= self.y1 + self.h1);
        self.contains = contains as u16;
        self.contains
    }
}
