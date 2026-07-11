//! Returns 1 if point (px, py) lies within or on a circle centered at (cx, cy) with radius r, tested via squared distance (px-cx)^2 + (py-cy)^2 <= r*r -- no sqrt, exact, same u32-widening trick euclid_sq/aabb_intersect rely on.
//! tags: grid, circle, point, contains, spatial, bounds, hit-test, distance, squared, no-sqrt
//! entry: PointInCircle::run
struct PointInCircle { px: u16, py: u16, cx: u16, cy: u16, r: u16, inside: u16 }
impl PointInCircle {
    fn run(&mut self) -> u16 {
        let dx = iabs_diff(self.px, self.cx);
        let dy = iabs_diff(self.py, self.cy);
        let dist_sq = dx as u32 * dx as u32 + dy as u32 * dy as u32;
        let r_sq = self.r as u32 * self.r as u32;
        let v = (dist_sq <= r_sq) as u16;
        self.inside = v;
        v
    }
}
