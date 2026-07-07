//! Pack two byte values into one u16: (hi << 8) | lo. Each input masked to its low byte, so out-of-range inputs stay defined.
//! tags: pack, byte, encode, combine, bits, high, low, hi, lo
fn run(hi: u16, lo: u16) -> u16 {
    ((hi & 0xFFu16) << 8u16) | (lo & 0xFFu16)
}
