//! Checked signed subtraction a - b of two i16 values, returned as i16 -- computed via sign-magnitude as add_i16(a, -b) by flipping b's sign flag before combining (never by native i16 subtraction or negate_i16): the sign-preserving sibling of abs_diff_i16, which throws the sign away and returns only the u16 magnitude, and the raw signed difference lerp_i16 computes internally but never exposes standalone.
//! tags: subtract, sub, minus, difference, delta, signed, i16, wide, u32, checked, escalate
//! limits: escalates (halt 0xFF05, needs_wider_math) if a - b does not fit back in i16 (e.g. a=i16::MAX, b=i16::MIN)
fn i16_mag(v: i16) -> u32 {
    if v < 0i16 { (0u16.wrapping_sub(v as u16)) as u32 } else { v as u16 as u32 }
}
fn i16_neg(v: i16) -> u16 {
    if v < 0i16 { 1u16 } else { 0u16 }
}
fn run(a: i16, b: i16) -> i16 {
    let a_mag = i16_mag(a);
    let a_neg = i16_neg(a);
    let b_mag = i16_mag(b);
    // sign of -b: flip b's sign flag, never negate b itself.
    let neg_b_neg = 1u16 - i16_neg(b);

    // diff = a + (-b), the smag_add shape (same pattern abs_diff_i16/lerp_i16 use).
    let mut diff_mag = 0u32;
    let mut diff_neg = 0u16;
    if a_neg == neg_b_neg {
        diff_mag = add_checked_u32(a_mag, b_mag);
        diff_neg = a_neg;
    } else if a_mag >= b_mag {
        diff_mag = a_mag - b_mag;
        diff_neg = if diff_mag == 0u32 { 0u16 } else { a_neg };
    } else {
        diff_mag = b_mag - a_mag;
        diff_neg = neg_b_neg;
    }

    // i16 holds magnitudes 0..=32768, but 32768 is only valid when negative (i16::MIN).
    if diff_mag > 32768u32 { halt(0xFF05u16); }
    if diff_mag == 32768u32 && diff_neg == 0u16 { halt(0xFF05u16); }

    if diff_neg == 1u16 {
        (0u16.wrapping_sub(diff_mag as u16)) as i16
    } else {
        diff_mag as u16 as i16
    }
}
