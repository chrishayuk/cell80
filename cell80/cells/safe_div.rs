//! Integer divide a / b, returning 0 when b == 0 (no divide-by-zero).
//! tags: math, arithmetic, divide, division, quotient, safe
fn run(a: u16, b: u16) -> u16 { if b != 0u16 { a / b } else { 0u16 } }
