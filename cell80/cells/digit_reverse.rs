//! Reverse the decimal digits of n (e.g. 123 -> 321; trailing zeros drop, so 120 -> 21).
//! tags: reverse, flip, mirror, digit-reverse, number, digits, decimal, math
//! limits: escalates (halt 0xFF05, needs_wider_math) if the reversed value would exceed 65535
fn run(n: u16) -> u16 {
    let mut v = n;
    let mut r = 0u32;
    while v != 0u16 {
        r = r * 10u32 + (v % 10u16) as u32;
        v = v / 10u16;
    }
    if r > 65535u32 { halt(0xFF05u16); }
    r as u16
}
