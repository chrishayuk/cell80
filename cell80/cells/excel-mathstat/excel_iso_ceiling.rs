//! Excel-compatible ISO.CEILING(number, [significance=1]): rounds number UP toward +infinity to the nearest multiple of significance's magnitude, discarding significance's own sign entirely -- Excel documents this as behaviorally identical to CEILING.PRECISE (excel_ceiling_precise.rs, this pack's checked-int/u32 sign-magnitude sibling, integer-significance only), but that cell's tier has no way to represent a fractional significance (e.g. 0.25 or 0.05, exactly the kind ISO.CEILING is routinely called with), so this cell is authored one tier down at signed Q8.8 fixed-point instead (int_to_q8_i16/q_mul_i16/q_div_i16/q_to_int_i16's own encode/decode/sign-magnitude conventions), taking number and significance pre-encoded; the core step is div_ceil_u32's own q+1-if-remainder ceiling idiom generalized to a signed dividend by noticing that i16_mag's truncating (toward-zero) integer division already IS the +infinity-directed ceiling once number is negative, and only needs the familiar +1 nudge when number is non-negative.
//! tags: excel, iso.ceiling, isoceiling, ceiling, ceiling-precise, round-up, toward-positive-infinity, multiple, significance, fractional-significance, fixed-point, q8.8, signed, i16, sign-magnitude, math-trig
//! limits: significance's sign is ignored entirely -- only its magnitude is used, matching Excel's own ISO.CEILING/CEILING.PRECISE convention (unlike CEILING.MATH's sign-aware mode flag); returns 0 (Q8.8 zero) if significance's magnitude is 0, matching Excel's ISO.CEILING(number, 0) = 0 rather than dividing by zero; the omitted-significance default of 1 is the caller's responsibility to substitute (pass significance_q8 = 256, Q8.8 for 1.0) before calling, the same convention excel_rate.rs documents for its own omittable args; escalates (halt 0xFF05, needs_wider_math) if the rounded-up magnitude doesn't fit back into i16 (> 32767 for a non-negative result, > 32768 for a negative one)
fn i16_mag(v: i16) -> u32 {
    if v < 0i16 { (0u16.wrapping_sub(v as u16)) as u32 } else { v as u16 as u32 }
}
fn i16_neg(v: i16) -> u16 {
    if v < 0i16 { 1u16 } else { 0u16 }
}
fn run(number_q8: i16, significance_q8: i16) -> i16 {
    let mag_s = i16_mag(significance_q8);
    if mag_s == 0u32 {
        0i16
    } else {
        let mag_n = i16_mag(number_q8);
        let neg_n = i16_neg(number_q8);
        let mut q = mag_n / mag_s;
        let rem = mag_n - q * mag_s;
        if neg_n == 0u16 {
            if rem != 0u32 {
                q = q + 1u32;
            }
        }
        let result_mag = q * mag_s;
        let limit = 32767u32 + neg_n as u32;
        if result_mag > limit {
            halt(0xFF05u16);
        }
        if result_mag == 0u32 {
            0i16
        } else if neg_n == 1u16 {
            (0u16.wrapping_sub(result_mag as u16)) as i16
        } else {
            result_mag as u16 as i16
        }
    }
}
