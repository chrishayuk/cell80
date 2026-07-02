//! Squared Euclidean distance between two grid points: dx*dx + dy*dy (no sqrt). Wide u32 dist field.
//! tags: grid, distance, euclidean, squared, spatial, magnitude
//! entry: Pts::run
struct Pts { x1: u16, y1: u16, x2: u16, y2: u16, dist: u32 }
impl Pts {
    fn run(&mut self) -> u16 {
        let dx = iabs_diff(self.x1, self.x2);
        let dy = iabs_diff(self.y1, self.y2);
        self.dist = dx as u32 * dx as u32 + dy as u32 * dy as u32;
        let mut r = self.dist as u16;
        if (self.dist >> 16u32) as u16 != 0u16 {
            r = 65535u16;
        }
        r
    }
}
