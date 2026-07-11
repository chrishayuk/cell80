//! Pack four byte values into one u32: (b3 << 24) | (b2 << 16) | (b1 << 8) | b0 -- the 4x8-bit rung of this pack's concatenation ladder that pack_u16_pair's 2x16-bit form doesn't reach; needs a state cell since four inputs exceed a free fn's 3-param cap.
//! tags: pack, byte, u32, encode, combine, bits, wide, concatenate, four
//! entry: PackBytes4::run
struct PackBytes4 { b3: u16, b2: u16, b1: u16, b0: u16, out: u32 }
impl PackBytes4 {
    fn run(&mut self) -> u16 {
        self.out = (self.b3 as u32 & 0xFFu32) << 24u32
            | (self.b2 as u32 & 0xFFu32) << 16u32
            | (self.b1 as u32 & 0xFFu32) << 8u32
            | (self.b0 as u32 & 0xFFu32);
        1u16
    }
}
