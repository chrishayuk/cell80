//! Manhattan distance between two grid points (typed state).
//! tags: grid, distance, spatial, score, navigation, taxicab, horizontal, vertical, axis-aligned, city-block
//! entry: Pts::run
struct Pts { x1: u16, y1: u16, x2: u16, y2: u16, dist: u16 }
impl Pts {
    fn run(&mut self) -> u16 {
        let dx = if self.x1 > self.x2 { self.x1 - self.x2 } else { self.x2 - self.x1 };
        let dy = if self.y1 > self.y2 { self.y1 - self.y2 } else { self.y2 - self.y1 };
        self.dist = dx + dy;
        self.dist
    }
}
