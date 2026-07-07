//! Reverse the 16 bits of x (bit 0 <-> bit 15, ...).
//! tags: bits, reverse, mirror, flip, bit-reversal, shuffle
fn run(x: u16) -> u16 {
    let mut v = x; let mut r = 0u16; let mut i = 0u16;
    while i < 16u16 { r = (r << 1u16) | (v & 1u16); v = v >> 1u16; i = i + 1u16; }
    r
}
