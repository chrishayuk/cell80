//! Excel MOD(number, divisor): remainder of number/divisor where the RESULT TAKES THE SIGN OF THE DIVISOR -- the opposite convention from Rust's native `%` (and most languages' truncating remainder), which instead takes the sign of the dividend -- composed from div_i16/mul_i16/sub_i16's own sign-magnitude techniques (each inlined here, since a cell cannot call another cell) to get the raw truncated remainder r = number - div_i16(number, divisor)*divisor, then Excel's own fixup: if r != 0 and sign(r) != sign(divisor), add divisor back -- distinct from mod_u32/safe_mod (unsigned-only, the result's sign is never in question) and from a plain truncating signed remainder (which would leave r uncorrected, carrying the dividend's sign instead).
//! tags: excel, mod, modulo, remainder, divisor-sign, floor-division, signed, i16, sign-magnitude, escalate, math-trig
//! limits: escalates (halt 0xFF05, needs_wider_math) if divisor == 0 (Excel's #DIV/0!), matching mod_u32/div_floor_u32's own zero-divisor convention; also escalates if the intermediate truncated quotient or product doesn't fit back in i16 (the same overflow guards div_i16/mul_i16 each carry standalone)
fn i16_mag(v: i16) -> u32 {
    if v < 0i16 { (0u16.wrapping_sub(v as u16)) as u32 } else { v as u16 as u32 }
}
fn i16_neg(v: i16) -> u16 {
    if v < 0i16 { 1u16 } else { 0u16 }
}
fn run(number: i16, divisor: i16) -> i16 {
    if divisor == 0i16 { halt(0xFF05u16); }

    let mag_n = i16_mag(number);
    let mag_d = i16_mag(divisor);
    let neg_n = i16_neg(number);
    let neg_d = i16_neg(divisor);

    // q = div_i16(number, divisor): truncating divide toward zero via sign-magnitude
    // (div_i16's own technique, inlined).
    let q_mag = mag_n / mag_d;
    let q_neg = if neg_n == neg_d { 0u16 } else { 1u16 };
    let q_limit = 32767u32 + q_neg as u32;
    if q_mag > q_limit { halt(0xFF05u16); }

    // p = mul_i16(q, divisor): checked signed multiply via sign-magnitude (mul_i16's own
    // technique, inlined) -- always fits u32 comfortably (q_mag <= mag_n <= 32768).
    let p_mag = q_mag * mag_d;
    let p_neg = if p_mag == 0u32 { 0u16 } else if q_neg == neg_d { 0u16 } else { 1u16 };
    if p_mag > 32768u32 { halt(0xFF05u16); }
    if p_mag == 32768u32 && p_neg == 0u16 { halt(0xFF05u16); }

    // r = sub_i16(number, p) = number + (-p): sign-magnitude combine (sub_i16's own
    // technique, inlined).
    let neg_p_flipped = 1u16 - p_neg;
    let mut r_mag = 0u32;
    let mut r_neg = 0u16;
    if neg_n == neg_p_flipped {
        r_mag = mag_n + p_mag;
        r_neg = neg_n;
    } else if mag_n >= p_mag {
        r_mag = mag_n - p_mag;
        r_neg = if r_mag == 0u32 { 0u16 } else { neg_n };
    } else {
        r_mag = p_mag - mag_n;
        r_neg = neg_p_flipped;
    }
    if r_mag > 32768u32 { halt(0xFF05u16); }
    if r_mag == 32768u32 && r_neg == 0u16 { halt(0xFF05u16); }

    // Excel's divisor-sign fixup: if r != 0 and sign(r) != sign(divisor), add divisor
    // back. By the truncating-remainder property |r| < |divisor|, whenever the signs
    // differ the sum reduces to a plain magnitude subtraction, taking the divisor's sign.
    if r_mag != 0u32 && r_neg != neg_d {
        r_mag = mag_d - r_mag;
        r_neg = neg_d;
    }

    let r16 = r_mag as u16;
    let result = if r_neg == 1u16 { (0u16.wrapping_sub(r16)) as i16 } else { r16 as i16 };
    result
}
