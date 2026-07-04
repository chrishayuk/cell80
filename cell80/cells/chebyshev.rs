//! Chebyshev (chessboard) distance between two grid points: max(|dx|, |dy|).
//! tags: grid, distance, chebyshev, chessboard, spatial, king-move, larger, max, maximum, axis
//! entry: Pts::run
struct Pts { x1: u16, y1: u16, x2: u16, y2: u16, dist: u16 }
impl Pts {
    fn run(&mut self) -> u16 {
        let dx = iabs_diff(self.x1, self.x2);
        let dy = iabs_diff(self.y1, self.y2);
        self.dist = imax(dx, dy);
        self.dist
    }
}
