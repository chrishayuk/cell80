//! The smallest AABB (ux,uy,uw,uh) containing both input AABBs (x1,y1,w1,h1) and (x2,y2,w2,h2) -- always defined, no overlap required, unlike aabb_intersect/aabb_contains which only test a relationship between two boxes.
//! tags: grid, aabb, rect, union, bounding, merge, spatial, bbox
//! entry: AabbUnion::run
struct AabbUnion { x1: u16, y1: u16, w1: u16, h1: u16, x2: u16, y2: u16, w2: u16, h2: u16, ux: u16, uy: u16, uw: u16, uh: u16 }
impl AabbUnion {
    fn run(&mut self) -> u16 {
        let ux = if self.x1 < self.x2 { self.x1 } else { self.x2 };
        let uy = if self.y1 < self.y2 { self.y1 } else { self.y2 };
        let right1 = self.x1 + self.w1;
        let right2 = self.x2 + self.w2;
        let right = if right1 > right2 { right1 } else { right2 };
        let bottom1 = self.y1 + self.h1;
        let bottom2 = self.y2 + self.h2;
        let bottom = if bottom1 > bottom2 { bottom1 } else { bottom2 };
        self.ux = ux;
        self.uy = uy;
        self.uw = right - ux;
        self.uh = bottom - uy;
        self.uw
    }
}
