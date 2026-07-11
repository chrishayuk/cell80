//! Hamming distance between two 16-bit values: the count of bit positions where a and b differ, popcount(a ^ b).
//! tags: bits, hamming, distance, xor, popcount, similarity, error-detection
fn run(a: u16, b: u16) -> u16 { let mut v = a ^ b; let mut c = 0u16; while v != 0u16 { c = c + (v & 1u16); v = v >> 1u16; } c }
