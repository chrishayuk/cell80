//! Least common multiple of two values (a/gcd*b; 0 if either is 0). u16 domain.
//! tags: number, lcm, multiple, common, divisor, math
fn run(a: u16, b: u16) -> u16 { let g = gcd(a, b); if g != 0u16 { a / g * b } else { 0u16 } }
