//! Number of decimal digits of n (0 has 1 digit).
//! tags: number, digits, length, decimal, count, magnitude
fn run(n: u16) -> u16 { let mut v = n; let mut c = 1u16; while v >= 10u16 { c = c + 1u16; v = v / 10u16; } c }
