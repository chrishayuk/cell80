//! Manhattan distance between two grid points with a wide u32 dist field: dx + dy stays exact where far-apart u16 coordinates would wrap a u16 result.
//! tags: grid, distance, spatial, score, navigation, wide, u32, large
//! entry: Pts::run
struct Pts { x1: u16, y1: u16, x2: u16, y2: u16, dist: u32 }
impl Pts {
    fn run(&mut self) -> u16 {
        let dx = iabs_diff(self.x1, self.x2);
        let dy = iabs_diff(self.y1, self.y2);
        self.dist = dx as u32 + dy as u32;
        if (self.dist >> 16u32) as u16 != 0u16 { 65535u16 } else { self.dist as u16 }
    }
}
