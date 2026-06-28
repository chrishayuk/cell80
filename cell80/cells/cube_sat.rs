//! Saturating cube: n*n*n, capped at 65535 (n >= 41 saturates).
//! tags: number, cube, power, multiply, saturating, math
fn run(n: u16) -> u16 { let mut r = 65535u16; if n <= 40u16 { r = n * n * n; } r }
