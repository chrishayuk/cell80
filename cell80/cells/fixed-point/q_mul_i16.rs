//! Signed Q8.8 fixed-point multiply of two i16 values via sign-magnitude: decompose each into (magnitude, sign) with i16_mag/i16_neg, multiply magnitudes and shift right 8 (mirroring q_mul's own (a*b)>>8 at wide width), sign is the XOR of the two input signs -- q_mul's signed counterpart, since q_mul only accepts unsigned u16 while q_sigmoid already established a signed i16 domain in this pack.
//! tags: fixed-point, q8.8, multiply, signed, i16, sign-magnitude, wide, u32, checked, escalate
//! limits: escalates (halt 0xFF05, needs_wider_math) if the scaled magnitude doesn't fit back into i16 (> 32767 for a positive result, > 32768 for a negative one)
fn i16_mag(v: i16) -> u32 {
    if v < 0i16 { (0u16.wrapping_sub(v as u16)) as u32 } else { v as u16 as u32 }
}
fn i16_neg(v: i16) -> u16 {
    if v < 0i16 { 1u16 } else { 0u16 }
}
fn run(a: i16, b: i16) -> i16 {
    let mag_a = i16_mag(a);
    let neg_a = i16_neg(a);
    let mag_b = i16_mag(b);
    let neg_b = i16_neg(b);
    let scaled = (mag_a * mag_b) >> 8u32;
    let neg = if neg_a == neg_b { 0u16 } else { 1u16 };
    let limit = 32767u32 + neg as u32;
    if scaled > limit { halt(0xFF05u16); }
    let mag16 = scaled as u16;
    let result = if neg == 1u16 { (0u16.wrapping_sub(mag16)) as i16 } else { mag16 as i16 };
    result
}
