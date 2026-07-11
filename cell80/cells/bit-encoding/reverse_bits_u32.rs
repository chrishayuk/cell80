//! Reverse all 32 bits of x (bit 0 <-> bit 31, ...) -- the u32-width sibling of reverse_bits, needing a state cell since the calling convention has no u32 free-fn parameters; distinct from swap_bytes (byte-order reordering, not bit-level reversal within each bit position).
//! tags: bits, reverse, mirror, flip, bit-reversal, shuffle, wide, u32
//! entry: ReverseBitsU32::run
struct ReverseBitsU32 { x: u32, out: u32 }
impl ReverseBitsU32 {
    fn run(&mut self) -> u16 {
        let mut v = self.x;
        let mut r = 0u32;
        let mut i = 0u16;
        while i < 32u16 {
            r = (r << 1u32) | (v & 1u32);
            v = v >> 1u32;
            i = i + 1u16;
        }
        self.out = r;
        1u16
    }
}
