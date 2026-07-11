//! Returns 1 if four side lengths (a, b, c, d) can be assembled into a valid (non-degenerate, simple) quadrilateral, i.e. each side is strictly less than the sum of the other three, else 0 -- the four-side polygon-inequality generalization of triangle_is_valid's own three-side triangle-inequality check.
//! tags: geometry, quadrilateral, validate, predicate, inequality, polygon, math
//! entry: QuadIsValid::run
struct QuadIsValid { a: u16, b: u16, c: u16, d: u16, valid: u16 }
impl QuadIsValid {
    fn run(&mut self) -> u16 {
        let aw = self.a as u32;
        let bw = self.b as u32;
        let cw = self.c as u32;
        let dw = self.d as u32;
        let mut v = 0u16;
        if aw < bw + cw + dw && bw < aw + cw + dw && cw < aw + bw + dw && dw < aw + bw + cw {
            v = 1u16;
        }
        self.valid = v;
        v
    }
}
