//! Saturating multiply: a * b, capped at 65535 instead of wrapping.
//! tags: math, arithmetic, multiply, product, saturating, safe
fn run(a: u16, b: u16) -> u16 {
    let mut r = 0u16;
    if a != 0u16 {
        if b > 65535u16 / a { r = 65535u16; } else { r = a * b; }
    }
    r
}
