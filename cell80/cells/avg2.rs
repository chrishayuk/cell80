//! Average of two values, (a + b) / 2, computed without overflow.
//! tags: math, average, mean, midpoint, halfway
fn run(a: u16, b: u16) -> u16 { (a & b) + ((a ^ b) >> 1u16) }
