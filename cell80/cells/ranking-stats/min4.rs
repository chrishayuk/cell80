//! Smallest of four values — the four-operand sibling of min3 (mirrors sum4's precedent for arity-4 in this pack).
//! tags: min, minimum, smallest, least, extremum, four
//! entry: Min4::run
struct Min4 { a: u16, b: u16, c: u16, d: u16 }
impl Min4 {
    fn run(&mut self) -> u16 { imin(imin(imin(self.a, self.b), self.c), self.d) }
}
