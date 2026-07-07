//! Population count: the number of set bits in a 16-bit value.
//! tags: bits, popcount, count, ones, hamming-weight, bitcount
fn run(x: u16) -> u16 { let mut v = x; let mut c = 0u16; while v != 0u16 { c = c + (v & 1u16); v = v >> 1u16; } c }
