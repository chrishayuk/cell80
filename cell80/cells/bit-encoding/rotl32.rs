//! Rotate the 32 bits of x left by n (n mod 32) -- the u32-width sibling of rotl16, built from a bounded loop of single-bit rotations since this dialect's u32 shifts take only constant-literal amounts, unlike rotl16's single variable-shift expression.
//! tags: bits, rotate, left, rol, shift, circular, wide, u32
//! entry: Rotl32::run
struct Rotl32 { x: u32, n: u16, out: u32 }
impl Rotl32 {
    fn run(&mut self) -> u16 {
        let s = self.n & 31u16;
        let mut v = self.x;
        let mut i = 0u16;
        while i < s {
            let top = v >> 31u32;
            v = (v << 1u32) | top;
            i = i + 1u16;
        }
        self.out = v;
        1u16
    }
}
