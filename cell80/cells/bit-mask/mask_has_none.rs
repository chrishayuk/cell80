//! Returns 1 if x has NONE of mask's bits set: (x & mask) == 0, else 0 -- the exact logical complement of mask_has_any.
//! tags: bits, mask, flags, none, disjoint, excludes
fn run(x: u16, mask: u16) -> u16 { ((x & mask) == 0u16) as u16 }
