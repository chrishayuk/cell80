//! Decode a signed Q8.8 fixed-point value back to a plain signed integer via an arithmetic (sign-propagating) right shift by 8 (x >> 8), which floors toward negative infinity for negatives -- int_to_q8_i16's missing decode counterpart, genuinely distinct from high_byte (a logical shift on u16, wrong for a negative i16 bit pattern) since i16's >> in this dialect sign-extends instead.
//! tags: fixed-point, q8.8, convert, decode, scale, integer, signed, i16, shift
fn run(x: i16) -> i16 {
    x >> 8i16
}
