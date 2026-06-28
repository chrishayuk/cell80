//! Returns 1 if bit number `bit` of x is set, else 0.
//! tags: bits, test, get, flag, is-set, check
fn run(x: u16, bit: u16) -> u16 { (x >> bit) & 1u16 }
