//! Low nibble of x (x & 0xF) -- the low-nibble counterpart to nibble_hi, distinct from low_byte's byte-level mask (x & 0xFF).
//! tags: bits, nibble, low, mask, extract, unpack, lo
fn run(x: u16) -> u16 { x & 0xFu16 }
