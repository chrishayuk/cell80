//! Number of bits needed to represent x: index of the highest set bit + 1 (0 for x == 0).
//! tags: bits, bit-length, msb, highest-bit, log2, magnitude
fn run(x: u16) -> u16 { let mut v = x; let mut c = 0u16; while v != 0u16 { c = c + 1u16; v = v >> 1u16; } c }
