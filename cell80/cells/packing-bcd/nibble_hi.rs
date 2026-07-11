//! Extract the high nibble of the low byte of x: (x >> 4) & 0xF, the unpacking counterpart pack_nibbles lacks.
//! tags: nibble, unpack, extract, bits, shift, mask, high
fn run(x: u16) -> u16 {
    (x >> 4u16) & 0xFu16
}
