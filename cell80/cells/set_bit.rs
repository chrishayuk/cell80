//! Set bit number `bit` of x to 1.
//! tags: bits, set, enable, flag, or, on
fn run(x: u16, bit: u16) -> u16 { x | (1u16 << bit) }
