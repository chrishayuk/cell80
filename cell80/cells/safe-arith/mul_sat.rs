//! Saturating multiply: a * b, capped at 65535 instead of wrapping.
//! tags: math, arithmetic, multiply, product, saturating, safe
fn run(a: u16, b: u16) -> u16 {
    if a == 0u16 { 0u16 } else if b > 65535u16 / a { 65535u16 } else { a * b }
}
