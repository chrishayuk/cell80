//! Split a packed u32 back into its high and low u16 halves — the inverse of pack_u16_pair, mirroring the morton_encode/morton_decode round-trip-pair convention.
//! tags: unpack, split, u32, u16, high, low, hi, lo, decode, wide
//! entry: UnpackU16Pair::run
struct UnpackU16Pair { in_val: u32, hi: u16, lo: u16 }
impl UnpackU16Pair {
    fn run(&mut self) -> u16 {
        self.hi = (self.in_val >> 16u32) as u16;
        self.lo = (self.in_val & 0x0000FFFFu32) as u16;
        1u16
    }
}
