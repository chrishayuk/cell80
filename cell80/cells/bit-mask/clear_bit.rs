//! Clear bit number `bit` of x to 0.
//! tags: bits, clear, unset, disable, flag, off
fn run(x: u16, bit: u16) -> u16 { x ^ (x & (1u16 << bit)) }
