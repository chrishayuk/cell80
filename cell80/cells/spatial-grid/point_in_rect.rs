//! Returns 1 if point (px, py) is inside rect (rx, ry, rw, rh) — half-open: [rx, rx+rw) x [ry, ry+rh) — else 0.
//! tags: grid, rect, point, contains, spatial, bounds, hit-test
//! entry: PointInRect::run
struct PointInRect { px: u16, py: u16, rx: u16, ry: u16, rw: u16, rh: u16, inside: u16 }
impl PointInRect {
    fn run(&mut self) -> u16 {
        let inside = (self.px >= self.rx)
            && (self.px < self.rx + self.rw)
            && (self.py >= self.ry)
            && (self.py < self.ry + self.rh);
        self.inside = inside as u16;
        self.inside
    }
}
