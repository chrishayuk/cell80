//! Saturating add: a + b, capped at 65535 instead of wrapping.
//! tags: math, arithmetic, add, sum, saturating, safe
fn run(a: u16, b: u16) -> u16 { let s = a.wrapping_add(b); let mut r = s; if s < a { r = 65535u16; } r }
