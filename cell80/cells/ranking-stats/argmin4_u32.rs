//! Index (0, 1, 2, or 3) of the smallest of four values at wide u32 width; ties -> lowest index — the wide sibling of argmin4, mirroring argmin3_u32's structure one level deeper.
//! tags: argmin, index, which, smallest, choose, select, wide, u32, large, four
//! entry: Argmin4Wide::run
struct Argmin4Wide { a: u32, b: u32, c: u32, d: u32 }
impl Argmin4Wide {
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
