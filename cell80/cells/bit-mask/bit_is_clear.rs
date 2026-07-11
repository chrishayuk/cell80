//! Returns 1 if bit number `bit` of x is NOT set, else 0: ((x >> bit) & 1u16) == 0u16 -- the exact logical complement of bit_is_set.
//! tags: bits, test, get, flag, is-clear, check, complement
fn run(x: u16, bit: u16) -> u16 { (((x >> bit) & 1u16) == 0u16) as u16 }
