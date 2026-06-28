//! Saturating subtract: a - b, floored at 0 when b > a.
//! tags: math, arithmetic, subtract, minus, saturating, safe
fn run(a: u16, b: u16) -> u16 { let mut r = 0u16; if a >= b { r = a - b; } r }
