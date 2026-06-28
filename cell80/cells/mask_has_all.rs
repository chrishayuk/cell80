//! Returns 1 if x has ALL bits of mask set: (x & mask) == mask.
//! tags: bits, mask, flags, all, contains, has
fn run(x: u16, mask: u16) -> u16 { ((x & mask) == mask) as u16 }
