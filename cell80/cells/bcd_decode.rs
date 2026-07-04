//! Decode a packed BCD byte (tens in the high nibble, units in the low nibble) back to its binary value.
//! tags: bcd, decimal, unpack, decode, binary-coded-decimal
fn run(bcd: u16) -> u16 {
    ((bcd >> 4u16) * 10u16) + (bcd & 0xFu16)
}
