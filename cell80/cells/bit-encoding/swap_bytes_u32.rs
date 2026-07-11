//! Reverse the byte order of a full 32-bit value (endian-swap all four bytes: b0 b1 b2 b3 -> b3 b2 b1 b0) -- the u32-width sibling of swap_bytes's 16-bit byte-swap, needing a u32 state field since the calling convention has no u32 free-fn parameters; distinct from reverse_bits_u32 in granularity (whole bytes moved, not individual bits).
//! tags: bits, byte, swap, endian, reverse, shuffle, wide, u32
//! entry: SwapBytesU32::run
struct SwapBytesU32 { x: u32, out: u32 }
impl SwapBytesU32 {
    fn run(&mut self) -> u16 {
        let b0 = self.x & 0xFFu32;
        let b1 = (self.x >> 8u32) & 0xFFu32;
        let b2 = (self.x >> 16u32) & 0xFFu32;
        let b3 = (self.x >> 24u32) & 0xFFu32;
        self.out = (b0 << 24u32) | (b1 << 16u32) | (b2 << 8u32) | b3;
        1u16
    }
}
