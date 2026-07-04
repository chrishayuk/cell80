//! Saturating cube: n*n*n, capped at 65535 (n >= 41 saturates).
//! tags: number, cube, power, multiply, saturating, math
fn run(n: u16) -> u16 { if n <= 40u16 { n * n * n } else { 65535u16 } }
