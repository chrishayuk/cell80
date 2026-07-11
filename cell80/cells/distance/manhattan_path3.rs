//! Total Manhattan path length across three consecutive grid points (x1,y1)->(x2,y2)->(x3,y3): the two segment distances summed into a wide u32 dist field, saturating the u16 return.
//! tags: grid, distance, spatial, route, path, waypoint, navigation, wide, u32, multi-hop
//! entry: Path3::run
struct Path3 { x1: u16, y1: u16, x2: u16, y2: u16, x3: u16, y3: u16, dist: u32 }
impl Path3 {
    fn run(&mut self) -> u16 {
        let dx1 = iabs_diff(self.x1, self.x2);
        let dy1 = iabs_diff(self.y1, self.y2);
        let dx2 = iabs_diff(self.x2, self.x3);
        let dy2 = iabs_diff(self.y2, self.y3);
        self.dist = dx1 as u32 + dy1 as u32 + dx2 as u32 + dy2 as u32;
        if (self.dist >> 16u32) as u16 != 0u16 { 65535u16 } else { self.dist as u16 }
    }
}
