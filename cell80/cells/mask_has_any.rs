//! Returns 1 if x has ANY bit of mask set: (x & mask) != 0.
//! tags: bits, mask, flags, any, overlap, intersects
fn run(x: u16, mask: u16) -> u16 { ((x & mask) != 0u16) as u16 }
