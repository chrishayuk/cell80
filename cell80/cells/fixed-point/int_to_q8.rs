//! Convert a plain unsigned integer into Q8.8 fixed-point representation (x << 8) -- the encode step every other fixed-point cell (q_mul, q_div, q_lerp, q_sigmoid, q_sqrt) assumes has already happened to its inputs.
//! tags: fixed-point, q8.8, convert, encode, scale, integer, checked
//! limits: escalates (halt 0xFF05, needs_wider_math) if x > 255, since Q8.8's 8 integer bits can't hold a larger whole-number part without silently losing high bits
fn run(x: u16) -> u16 {
    if x > 255u16 { halt(0xFF05u16); }
    x << 8u16
}
