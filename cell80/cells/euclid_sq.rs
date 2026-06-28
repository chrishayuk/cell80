//! Squared Euclidean distance between two grid points: dx*dx + dy*dy (no sqrt). u16 domain.
//! tags: grid, distance, euclidean, squared, spatial, magnitude
//! entry: Pts::run
struct Pts { x1: u16, y1: u16, x2: u16, y2: u16, dist: u16 }
impl Pts {
    fn run(&mut self) -> u16 {
        let dx = iabs_diff(self.x1, self.x2);
        let dy = iabs_diff(self.y1, self.y2);
        self.dist = dx * dx + dy * dy;
        self.dist
    }
}
