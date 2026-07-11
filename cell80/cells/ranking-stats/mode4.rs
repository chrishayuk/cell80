//! Mode of four values: extends mode3's "first value found to repeat" convention one level -- a repeating anywhere in {b,c,d} wins first (covers majority-of-4-in-a and 2-2 ties by priority), then b repeating in {c,d}, then c repeating in d, defaulting to a if all four are distinct.
//! tags: mode, most-common, repeated, four, stat, majority, tie
//! entry: Mode4::run
struct Mode4 { a: u16, b: u16, c: u16, d: u16 }
impl Mode4 {
    fn run(&mut self) -> u16 {
        if self.a == self.b || self.a == self.c || self.a == self.d {
            self.a
        } else if self.b == self.c || self.b == self.d {
            self.b
        } else if self.c == self.d {
            self.c
        } else {
            self.a
        }
    }
}
