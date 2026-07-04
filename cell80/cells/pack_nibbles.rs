//! Pack two 4-bit nibbles into one byte: (hi << 4) | lo. Each input masked to its low nibble.
//! tags: pack, nibble, encode, combine, bits
fn run(hi: u16, lo: u16) -> u16 {
    ((hi & 0xFu16) << 4u16) | (lo & 0xFu16)
}
