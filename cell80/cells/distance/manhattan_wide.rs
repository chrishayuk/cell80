//! Manhattan distance between two grid points: dx + dy, into a wide u32 dist field so two extreme-apart u16 coordinates can't silently wrap past u16 the way manhattan's u16 dist field can.
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
