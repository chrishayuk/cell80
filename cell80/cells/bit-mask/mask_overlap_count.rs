//! Count of bit positions where a and b are both set: popcount(a & b) -- distinct from mask_intersection (returns the mask itself, not a scalar count) and hamming_distance16 (counts bits that differ, not bits that agree-and-are-set).
//! tags: bits, mask, intersection, popcount, count, overlap, agreement, and
fn run(a: u16, b: u16) -> u16 { let mut v = a & b; let mut c = 0u16; while v != 0u16 { c = c + (v & 1u16); v = v >> 1u16; } c }
