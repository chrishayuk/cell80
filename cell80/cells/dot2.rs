//! Dot product of two 2D vectors (ax, ay) and (bx, by): ax*bx + ay*by.
//! tags: vector, dot-product, math, score, similarity
//! entry: Dot2::run
struct Dot2 { ax: u16, ay: u16, bx: u16, by: u16, dot: u16 }
impl Dot2 {
    fn run(&mut self) -> u16 {
        self.dot = self.ax * self.bx + self.ay * self.by;
        self.dot
    }
}
