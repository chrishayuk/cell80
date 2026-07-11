//! Checked three-way signed add a + b + c via sign-magnitude, combining pairwise (a+b, then the running sum + c) and escalating only if the final combined magnitude doesn't fit back in i16 -- the three-operand sibling of add_i16, mirroring the library's own add->add3 arity precedent (sum3/sum4, add_checked_u32->add3_checked_u32) applied to the signed i16 domain.
//! tags: add, addition, sum, plus, signed, i16, three, add3, triple, wide, u32, checked, escalate
//! limits: escalates (halt 0xFF05, needs_wider_math) if a+b+c's true magnitude doesn't fit back in i16 (e.g. a=i16::MAX, b=i16::MAX, c=1)
fn i16_mag(v: i16) -> u32 {
    if v < 0i16 { (0u16.wrapping_sub(v as u16)) as u32 } else { v as u16 as u32 }
}
fn i16_neg(v: i16) -> u16 {
    if v < 0i16 { 1u16 } else { 0u16 }
}
fn run(a: i16, b: i16, c: i16) -> i16 {
    let a_mag = i16_mag(a);
    let a_neg = i16_neg(a);
    let b_mag = i16_mag(b);
    let b_neg = i16_neg(b);
    let c_mag = i16_mag(c);
    let c_neg = i16_neg(c);

    // ab = a + b (the smag_add shape, same pattern add_i16/lerp_i16 use).
    let mut ab_mag = 0u32;
    let mut ab_neg = 0u16;
    if a_neg == b_neg {
        ab_mag = add_checked_u32(a_mag, b_mag);
        ab_neg = a_neg;
    } else if a_mag >= b_mag {
        ab_mag = a_mag - b_mag;
        ab_neg = if ab_mag == 0u32 { 0u16 } else { a_neg };
    } else {
        ab_mag = b_mag - a_mag;
        ab_neg = b_neg;
    }

    // sum = ab + c (the smag_add shape again) -- no i16-fit check yet, only the final result must fit.
    let mut sum_mag = 0u32;
    let mut sum_neg = 0u16;
    if ab_neg == c_neg {
        sum_mag = add_checked_u32(ab_mag, c_mag);
        sum_neg = ab_neg;
    } else if ab_mag >= c_mag {
        sum_mag = ab_mag - c_mag;
        sum_neg = if sum_mag == 0u32 { 0u16 } else { ab_neg };
    } else {
        sum_mag = c_mag - ab_mag;
        sum_neg = c_neg;
    }

    // i16 holds magnitudes 0..=32768, but 32768 is only valid when negative (i16::MIN).
    if sum_mag > 32768u32 { halt(0xFF05u16); }
    if sum_mag == 32768u32 && sum_neg == 0u16 { halt(0xFF05u16); }

    if sum_neg == 1u16 {
        (0u16.wrapping_sub(sum_mag as u16)) as i16
    } else {
        sum_mag as u16 as i16
    }
}
