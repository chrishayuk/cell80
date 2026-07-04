//! Q8.8 fixed-point divide: (a << 8) / b, returning 0 when b == 0 (no divide-by-zero).
//! tags: fixed-point, q8.8, divide, scale, math, wide, safe
fn run(a: u16, b: u16) -> u16 {
    if b != 0u16 {
        (((a as u32) << 8u32) / b as u32) as u16
    } else {
        0u16
    }
}
