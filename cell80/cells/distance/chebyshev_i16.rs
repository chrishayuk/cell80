//! Chebyshev (chessboard) distance between two grid points with signed (i16) coordinates: max(|dx|, |dy|), each coordinate difference computed via an excess-32768 shift feeding the shared iabs_diff kernel (the manhattan_i16 technique), then the shared imax kernel -- the signed sibling chebyshev lacks, since its u16-only fields can't take an origin-centered coordinate at all; distinct from manhattan_i16 by taking the max rather than the sum, so its dist field stays a plain u16 (no widening needed, since max of two u16 values never exceeds either input).
//! tags: grid, distance, spatial, score, navigation, signed, i16, chebyshev, chessboard, king-move, max, maximum
//! entry: PtsSigned::run
struct PtsSigned { x1: i16, y1: i16, x2: i16, y2: i16, dist: u16 }
impl PtsSigned {
    fn run(&mut self) -> u16 {
        let sx1 = (self.x1 as u16).wrapping_add(32768u16);
        let sx2 = (self.x2 as u16).wrapping_add(32768u16);
        let sy1 = (self.y1 as u16).wrapping_add(32768u16);
        let sy2 = (self.y2 as u16).wrapping_add(32768u16);
        let dx = iabs_diff(sx1, sx2);
        let dy = iabs_diff(sy1, sy2);
        self.dist = imax(dx, dy);
        self.dist
    }
}
