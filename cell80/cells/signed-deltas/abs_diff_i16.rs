//! Absolute difference |a - b| for two signed 16-bit inputs, returned as u16 -- tracked via sign-magnitude subtraction rather than a raw i16 subtract, since a=i16::MAX, b=i16::MIN would overflow i16 by one before abs() could be taken; the signed-input sibling of abs_diff (u16) and abs_diff_u32 (wide).
//! tags: abs, diff, absolute, difference, signed, i16, delta, wide, u32, checked
//! limits: calls add_checked_u32 for the same checked-add honesty its sign-magnitude siblings use, but the 0xFF05 escalation is unreachable for i16 inputs -- the largest possible magnitude sum is 32768+32767=65535, which fits exactly in the u16 return
fn i16_mag(v: i16) -> u32 {
    if v < 0i16 { (0u16.wrapping_sub(v as u16)) as u32 } else { v as u16 as u32 }
}
fn i16_neg(v: i16) -> u16 {
    if v < 0i16 { 1u16 } else { 0u16 }
}
fn run(a: i16, b: i16) -> u16 {
    let a_mag = i16_mag(a);
    let a_neg = i16_neg(a);
    let b_mag = i16_mag(b);
    let b_neg_f = 1u16 - i16_neg(b);

    // diff = a - b, tracked as (magnitude, sign); we only need the magnitude, which is |a - b|.
    let mut diff_mag = 0u32;
    if a_neg == b_neg_f {
        diff_mag = add_checked_u32(a_mag, b_mag);
    } else if a_mag >= b_mag {
        diff_mag = a_mag - b_mag;
    } else {
        diff_mag = b_mag - a_mag;
    }
    diff_mag as u16
}
