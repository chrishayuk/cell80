//! Split a packed u32 back into its four constituent bytes b3..b0 via shifts and masks -- the inverse of pack_bytes4, mirroring the pack_u16_pair/unpack_u16_pair round-trip-pair convention.
//! tags: unpack, split, u32, byte, high, low, decode, wide, four, bits
//! entry: UnpackBytes4::run
struct UnpackBytes4 { in_val: u32, b3: u16, b2: u16, b1: u16, b0: u16 }
impl UnpackBytes4 {
    fn run(&mut self) -> u16 {
        self.b3 = ((self.in_val >> 24u32) & 0xFFu32) as u16;
        self.b2 = ((self.in_val >> 16u32) & 0xFFu32) as u16;
        self.b1 = ((self.in_val >> 8u32) & 0xFFu32) as u16;
        self.b0 = (self.in_val & 0xFFu32) as u16;
        1u16
    }
}
