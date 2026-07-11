//! Pack two u16 halves into one u32: (hi << 16) | lo — the u32-width generalization of pack_u8's (hi << 8) | lo, one rung further up the same concatenation ladder than pack_u8/pack_nibbles reach; needs a u32 state field since two full u16s produce 32 bits.
//! tags: pack, u16, u32, encode, combine, bits, high, low, hi, lo, wide, concatenate
//! entry: PackU16Pair::run
struct PackU16Pair { hi: u16, lo: u16, out: u32 }
impl PackU16Pair {
    fn run(&mut self) -> u16 {
        self.out = (self.hi as u32) << 16u32 | (self.lo as u32);
        1u16
    }
}
