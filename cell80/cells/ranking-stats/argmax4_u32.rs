//! Index (0, 1, 2, or 3) of the largest of four u32 values; ties -> lowest index — the wide sibling of argmax4, extending argmax3_u32's if-chain one level deeper exactly as argmax4 extends argmax3's.
//! tags: argmax, index, which, largest, choose, select, wide, u32, large, four
//! entry: Argmax4Wide::run
struct Argmax4Wide { a: u32, b: u32, c: u32, d: u32 }
impl Argmax4Wide {
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
