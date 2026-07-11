//! Spread of four values: max4 − min4 — the four-operand sibling of range3, one level deeper.
//! tags: range, spread, span, stat, four, extent
//! entry: Range4::run
struct Range4 { a: u16, b: u16, c: u16, d: u16 }
impl Range4 {
    fn run(&mut self) -> u16 {
        imax(imax(imax(self.a, self.b), self.c), self.d) - imin(imin(imin(self.a, self.b), self.c), self.d)
    }
}
