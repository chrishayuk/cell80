//! Rotate the 16 bits of x right by n (n taken mod 16).
//! tags: bits, rotate, right, ror, shift, circular
fn run(x: u16, n: u16) -> u16 { let s = n & 15u16; (x >> s) | (x << (16u16 - s & 15u16)) }
