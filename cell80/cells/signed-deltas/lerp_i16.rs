//! Linear interpolation from a to b by t (Q0.8 fraction, 0..256 = 0.0..1.0) for signed i16 endpoints: a + (b-a)*t/256, the fractional step truncated toward zero -- the signed sibling of q_lerp, computed via sign-magnitude throughout (never native i16 subtraction) since b-a can exceed i16's own representable range even when a and b are both valid i16 values (e.g. a=i16::MAX, b=i16::MIN, diff magnitude 65535). The long-open "overflow safety not yet worked out" blocker, closed by the sign-magnitude pattern this session's linear_solve_1var/linear_eq_holds proved out.
//! tags: fixed-point, q8.8, lerp, interpolate, blend, signed, i16, ema, moving-average, mix
fn i16_mag(v: i16) -> u32 {
    if v < 0i16 { (0u16.wrapping_sub(v as u16)) as u32 } else { v as u16 as u32 }
}
fn i16_neg(v: i16) -> u16 {
    if v < 0i16 { 1u16 } else { 0u16 }
}
fn run(a: i16, b: i16, t: u16) -> i16 {
    let a_mag = i16_mag(a);
    let a_neg = i16_neg(a);
    let b_mag = i16_mag(b);
    let b_neg = i16_neg(b);

    // diff = b - a: flip a's sign, then combine (the smag_sub shape).
    let a_neg_f = 1u16 - a_neg;
    let mut diff_mag = 0u32;
    let mut diff_neg = 0u16;
    if b_neg == a_neg_f {
        diff_mag = b_mag + a_mag;
        diff_neg = b_neg;
    } else if b_mag >= a_mag {
        diff_mag = b_mag - a_mag;
        diff_neg = if diff_mag == 0u32 { 0u16 } else { b_neg };
    } else {
        diff_mag = a_mag - b_mag;
        diff_neg = a_neg_f;
    }

    // step = diff * t / 256, magnitude truncated toward zero, diff's sign carried through.
    let step_mag = (diff_mag * (t as u32)) >> 8u32;

    // result = a + step (the smag_add shape).
    let mut result_mag = 0u32;
    let mut result_neg = 0u16;
    if a_neg == diff_neg {
        result_mag = a_mag + step_mag;
        result_neg = a_neg;
    } else if a_mag >= step_mag {
        result_mag = a_mag - step_mag;
        result_neg = if result_mag == 0u32 { 0u16 } else { a_neg };
    } else {
        result_mag = step_mag - a_mag;
        result_neg = diff_neg;
    }

    if result_neg == 1u16 {
        (0u16.wrapping_sub(result_mag as u16)) as i16
    } else {
        result_mag as u16 as i16
    }
}
