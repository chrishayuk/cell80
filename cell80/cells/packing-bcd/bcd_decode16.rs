//! Decode a four-digit packed-BCD u16 (one decimal digit per nibble) back to its binary value (0-9999) -- the inverse of bcd_encode16, mirroring the bcd_encode/bcd_decode pairing convention.
//! tags: bcd, decimal, unpack, decode, binary-coded-decimal, u16, nibble, four-digit
fn run(bcd: u16) -> u16 {
    (((bcd >> 12u16) & 0xFu16) * 1000u16) + (((bcd >> 8u16) & 0xFu16) * 100u16) + (((bcd >> 4u16) & 0xFu16) * 10u16) + (bcd & 0xFu16)
}
