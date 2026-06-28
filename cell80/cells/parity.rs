//! Parity: 1 if the number of set bits is odd, else 0.
//! tags: bits, parity, odd, xor, ones, checksum
fn run(x: u16) -> u16 { let mut v = x; let mut c = 0u16; while v != 0u16 { c = c + (v & 1u16); v = v >> 1u16; } c & 1u16 }
