//! Largest of four values — the four-operand sibling of max3, nested imax one level deeper.
//! tags: max, maximum, largest, greatest, extremum, four
//! entry: Max4::run
struct Max4 { a: u16, b: u16, c: u16, d: u16 }
impl Max4 {
    fn run(&mut self) -> u16 { imax(imax(imax(self.a, self.b), self.c), self.d) }
}
