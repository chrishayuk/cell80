//! Convert a plain signed integer into Q8.8 fixed-point representation (x << 8) -- the signed counterpart of int_to_q8 (which only accepts unsigned u16 and cannot represent negatives), needed as the encode step for this pack's already-signed cells (q_sigmoid, q_mul_i16, q_div_i16, clamp_i16) since none of them currently has one of their own.
//! tags: fixed-point, q8.8, convert, encode, scale, integer, signed, i16, checked
//! limits: escalates (halt 0xFF05, needs_wider_math) if x > 127 or x < -128, since Q8.8's 8 integer bits (signed) can't hold a larger whole-number part without silently losing high bits
fn run(x: i16) -> i16 {
    if x > 127i16 || x < -128i16 { halt(0xFF05u16); }
    x << 8i16
}
