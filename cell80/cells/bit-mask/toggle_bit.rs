//! Toggle (flip) bit number `bit` of x.
//! tags: bits, toggle, flip, xor, flag, invert
fn run(x: u16, bit: u16) -> u16 { x ^ (1u16 << bit) }
