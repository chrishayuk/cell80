//! Index (0, 1, 2, or 3) of the largest of four values; ties -> lowest index — the four-value sibling of argmax3, extending its if-chain one level deeper (distinct from max4/choose_best4, which return the value, not which slot holds it).
//! tags: argmax, index, which, largest, choose, select, four
//! entry: Argmax4::run
struct Argmax4 { a: u16, b: u16, c: u16, d: u16 }
impl Argmax4 {
    fn run(&mut self) -> u16 {
        if self.b > self.a {
            if self.c > self.b {
                if self.d > self.c { 3u16 } else { 2u16 }
            } else if self.d > self.b { 3u16 } else { 1u16 }
        } else if self.c > self.a {
            if self.d > self.c { 3u16 } else { 2u16 }
        } else if self.d > self.a { 3u16 } else { 0u16 }
    }
}
