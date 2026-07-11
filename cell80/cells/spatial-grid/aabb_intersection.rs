//! The actual overlapping rectangle (ix,iy,iw,ih) of two AABBs (x1,y1,w1,h1) and (x2,y2,w2,h2), plus a valid flag (0 when they don't truly overlap) -- unlike aabb_intersect's plain 0/1 verdict, this returns the intersection region itself.
//! tags: grid, aabb, rect, intersect, overlap, intersection, region, spatial, clip
//! entry: AabbIntersection::run
struct AabbIntersection { x1: u16, y1: u16, w1: u16, h1: u16, x2: u16, y2: u16, w2: u16, h2: u16, ix: u16, iy: u16, iw: u16, ih: u16, valid: u16 }
impl AabbIntersection {
    fn run(&mut self) -> u16 {
        let r1 = self.x1 + self.w1;
        let r2 = self.x2 + self.w2;
        let b1 = self.y1 + self.h1;
        let b2 = self.y2 + self.h2;

        let mut left = self.x1;
        if self.x2 > left { left = self.x2; }
        let mut top = self.y1;
        if self.y2 > top { top = self.y2; }
        let mut right = r1;
        if r2 < right { right = r2; }
        let mut bottom = b1;
        if b2 < bottom { bottom = b2; }

        let valid = (right > left) && (bottom > top);
        self.valid = valid as u16;

        let ix = if valid { left } else { 0u16 };
        let iy = if valid { top } else { 0u16 };
        let iw = if valid { right - left } else { 0u16 };
        let ih = if valid { bottom - top } else { 0u16 };
        self.ix = ix;
        self.iy = iy;
        self.iw = iw;
        self.ih = ih;
        self.valid
    }
}
