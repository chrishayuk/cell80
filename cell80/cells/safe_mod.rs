//! Remainder a % b, returning 0 when b == 0.
//! tags: math, arithmetic, modulo, remainder, mod, safe
fn run(a: u16, b: u16) -> u16 { let mut r = 0u16; if b != 0u16 { r = a % b; } r }
