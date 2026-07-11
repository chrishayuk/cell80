//! Bitwise complement of all 16 bits: x ^ 0xFFFF (unary NOT, the dialect's `!` is logical-not only).
//! tags: bits, mask, not, complement, invert, xor, flags
fn run(x: u16) -> u16 { x ^ 0xFFFFu16 }
