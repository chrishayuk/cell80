//! Sum of the decimal digits of n.
//! tags: number, digits, digit-sum, decimal, digital-root, math
fn run(n: u16) -> u16 { let mut v = n; let mut s = 0u16; while v != 0u16 { s = s + v % 10u16; v = v / 10u16; } s }
