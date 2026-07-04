//! Absolute value of a signed 16-bit value, returned as u16 (correctly handles i16::MIN, whose magnitude 32768 doesn't fit back in i16).
//! tags: abs, absolute, magnitude, signed, i16, delta
//! limits: none — the u16 return always fits, unlike a naive i16 negation of i16::MIN
fn run(x: i16) -> u16 {
    if x < 0i16 {
        0u16.wrapping_sub(x as u16)
    } else {
        x as u16
    }
}
