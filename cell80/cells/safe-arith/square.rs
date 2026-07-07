//! Saturating square: n * n, capped at 65535.
//! tags: math, square, power, multiply, saturating
fn run(n: u16) -> u16 { if n <= 255u16 { n * n } else { 65535u16 } }
