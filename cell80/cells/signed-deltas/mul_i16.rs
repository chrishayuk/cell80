//! Checked signed multiplication a*b of two i16 values via sign-magnitude: mag_a*mag_b always fits u32 (max 32768*32768), sign is the XOR of the two input signs, escalating if the product's magnitude doesn't fit back in i16 -- distinct from q_mul_i16 (Q8.8 scaled, shifts the product right 8) and smag_mul (a checked-arithmetic state cell taking pre-decomposed magnitude/sign pairs as fields); no raw unscaled i16*i16 signed multiply exists anywhere else in the library.
//! tags: multiply, multiplication, product, times, signed, i16, wide, u32, checked, escalate, sign-magnitude
//! limits: escalates (halt 0xFF05, needs_wider_math) if a * b does not fit back in i16 (e.g. a=i16::MIN, b=2i16)
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
    let b_neg = i16_neg(b);

    // prod = a_mag * b_mag: each magnitude is at most 32768, so the product is at
    // most 32768*32768 = 1_073_741_824, always fits u32 -- no checked-multiply needed.
    let prod_mag = a_mag * b_mag;
    let prod_neg = if prod_mag == 0u32 { 0u16 } else if a_neg == b_neg { 0u16 } else { 1u16 };

    // i16 holds magnitudes 0..=32768, but 32768 is only valid when negative (i16::MIN).
    if prod_mag > 32768u32 { halt(0xFF05u16); }
    if prod_mag == 32768u32 && prod_neg == 0u16 { halt(0xFF05u16); }

    if prod_neg == 1u16 {
        (0u16.wrapping_sub(prod_mag as u16)) as i16
    } else {
        prod_mag as u16 as i16
    }
}
