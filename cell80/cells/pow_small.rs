//! base raised to exp (saturating at 65535). 0^0 = 1.
//! tags: number, power, exponent, pow, saturating, math
fn run(base: u16, exp: u16) -> u16 {
    let mut r = 1u16;
    let mut i = 0u16;
    while i < exp {
        r = if base != 0u16 && r > 65535u16 / base { 65535u16 } else { r * base };
        i = i + 1u16;
    }
    r
}
