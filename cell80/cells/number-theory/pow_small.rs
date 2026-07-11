//! base raised to exp (saturating at 65535). 0^0 = 1.
//! tags: number, power, exponent, pow, saturating, math
fn run(base: u16, exp: u16) -> u16 {
    let mut r = 1u16;
    if base > 1u16 {
        // 65535 is absorbing for base >= 2, so the loop exits the step it
        // saturates — at most 16 multiplies, never the full exponent.
        let mut i = 0u16;
        while i < exp && r != 65535u16 {
            r = if r > 65535u16 / base { 65535u16 } else { r * base };
            i = i + 1u16;
        }
    } else if base == 0u16 && exp > 0u16 {
        r = 0u16;
    }
    r
}
