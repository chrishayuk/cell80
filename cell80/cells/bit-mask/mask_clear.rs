//! Clear every bit of `mask` from x: x & (mask ^ 0xFFFF), the mask-level generalization of clear_bit (AND-NOT / andn).
//! tags: bits, mask, clear, andn, and-not, disable, unset
fn run(x: u16, mask: u16) -> u16 { x & (mask ^ 0xFFFFu16) }
