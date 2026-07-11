//! Index (0, 1, 2, or 3) of the smallest of four values; ties -> lowest index — the four-value sibling of argmin3, extending its if-chain one level deeper (returns the winning slot, not the value, completing the argmax4/argmin4 pair).
//! tags: argmin, index, which, smallest, choose, select, four
//! entry: Argmin4::run
struct Argmin4 { a: u16, b: u16, c: u16, d: u16 }
impl Argmin4 {
    fn run(&mut self) -> u16 {
        if self.b < self.a {
            if self.c < self.b {
                if self.d < self.c { 3u16 } else { 2u16 }
            } else if self.d < self.b { 3u16 } else { 1u16 }
        } else if self.c < self.a {
            if self.d < self.c { 3u16 } else { 2u16 }
        } else if self.d < self.a { 3u16 } else { 0u16 }
    }
}
