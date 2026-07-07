//! Swap the high and low bytes of x ((x << 8) | (x >> 8)).
//! tags: bits, byte, swap, endian, reverse, shuffle
fn run(x: u16) -> u16 { (x << 8u16) | (x >> 8u16) }
