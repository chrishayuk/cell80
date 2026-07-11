//! Returns 1 if x is missing at least one bit of mask: (x & mask) != mask -- the exact logical complement of mask_has_all.
//! tags: bits, mask, flags, missing, incomplete, not_all
fn run(x: u16, mask: u16) -> u16 { ((x & mask) != mask) as u16 }
