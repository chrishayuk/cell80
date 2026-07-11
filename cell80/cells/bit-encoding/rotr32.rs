//! Rotate the 32 bits of x right by n (n taken mod 32) -- the u32-width sibling of rotr16, mirroring rotl32's left rotation; needs a state cell since the calling convention has no u32 free-fn parameters.
//! tags: bits, rotate, right, ror, shift, circular, wide, u32
//! entry: Rotr32::run
struct Rotr32 { x: u32, n: u16, out: u32 }
impl Rotr32 {
    fn run(&mut self) -> u16 {
        let s = self.n % 32u16;
        let mut v = self.x;
        let mut i = 0u16;
        while i < s {
            let bit0 = v & 1u32;
            v = (v >> 1u32) | (bit0 << 31u32);
            i = i + 1u16;
        }
        self.out = v;
        1u16
    }
}
