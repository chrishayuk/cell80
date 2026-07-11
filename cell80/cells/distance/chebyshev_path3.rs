//! Total Chebyshev ('king-move') path cost across three consecutive grid points (x1,y1)->(x2,y2)->(x3,y3): chebyshev(p1,p2) + chebyshev(p2,p3), summed into a wide u32 dist field -- the multi-hop sibling chebyshev/chebyshev_i16 lack; chebyshev's own single-max result never needs widening, but summing two maxes across a path can exceed u16, the same design point manhattan_path3 established for the sum metric.
//! tags: grid, distance, chebyshev, chessboard, king-move, spatial, route, path, waypoint, navigation, wide, u32, multi-hop
//! entry: Path3::run
struct Path3 { x1: u16, y1: u16, x2: u16, y2: u16, x3: u16, y3: u16, dist: u32 }
impl Path3 {
    fn run(&mut self) -> u16 {
        let dx1 = iabs_diff(self.x1, self.x2);
        let dy1 = iabs_diff(self.y1, self.y2);
        let c1 = imax(dx1, dy1);
        let dx2 = iabs_diff(self.x2, self.x3);
        let dy2 = iabs_diff(self.y2, self.y3);
        let c2 = imax(dx2, dy2);
        self.dist = c1 as u32 + c2 as u32;
        if (self.dist >> 16u32) as u16 != 0u16 { 65535u16 } else { self.dist as u16 }
    }
}
