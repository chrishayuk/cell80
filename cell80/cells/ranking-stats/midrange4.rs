//! Midrange of four values: (min4 + max4) / 2, via the same (lo & hi) + ((lo ^ hi) >> 1) trick midrange3 uses, now over imin/imax nested three deep.
//! tags: midrange, mid, average, four, stat, center
//! entry: Midrange4::run
struct Midrange4 { a: u16, b: u16, c: u16, d: u16 }
impl Midrange4 {
    fn run(&mut self) -> u16 {
        let lo = imin(imin(imin(self.a, self.b), self.c), self.d);
        let hi = imax(imax(imax(self.a, self.b), self.c), self.d);
        (lo & hi) + ((lo ^ hi) >> 1u16)
    }
}
