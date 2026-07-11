//! Plain signed integer division a/b of two i16 values via sign-magnitude (mag_a/mag_b, truncating toward zero), sign the XOR of the two input signs -- distinct from q_div_i16 (which scales by << 8 for a Q8.8 fixed-point result), this is the unscaled integer divide that completes add/sub/mul/div over this pack's signed domain.
//! tags: divide, signed, i16, sign-magnitude, integer, quotient, math
//! limits: escalates (halt 0xFF05, needs_wider_math) if a == i16::MIN and b == -1, since the true quotient 32768 has no representation in i16; returns 0 if b == 0
fn i16_mag(v: i16) -> u32 {
    if v < 0i16 { (0u16.wrapping_sub(v as u16)) as u32 } else { v as u16 as u32 }
}
fn i16_neg(v: i16) -> u16 {
    if v < 0i16 { 1u16 } else { 0u16 }
}
fn run(a: i16, b: i16) -> i16 {
    let mag_b = i16_mag(b);
    if mag_b != 0u32 {
        let mag_a = i16_mag(a);
        let neg_a = i16_neg(a);
        let neg_b = i16_neg(b);
        let quotient = mag_a / mag_b;
        let neg = if neg_a == neg_b { 0u16 } else { 1u16 };
        let limit = 32767u32 + neg as u32;
        if quotient > limit { halt(0xFF05u16); }
        let mag16 = quotient as u16;
        let result = if neg == 1u16 { (0u16.wrapping_sub(mag16)) as i16 } else { mag16 as i16 };
        result
    } else {
        0i16
    }
}
