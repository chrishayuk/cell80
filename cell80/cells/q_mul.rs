//! Q8.8 fixed-point multiply: (a * b) >> 8, computed wide so the 16.16 intermediate doesn't overflow.
//! tags: fixed-point, q8.8, multiply, scale, math, wide
//! limits: assumes the true product fits u16 once shifted back down (like percent/scale_percent)
fn run(a: u16, b: u16) -> u16 {
    ((a as u32 * b as u32) >> 8u32) as u16
}
