//! Take pct percent of a signed value, sign preserved: mag = i16_mag(value)*pct/100, saturated to i16's representable magnitude range (32767 if value is non-negative, 32768 if value is negative) -- the signed sibling of scale_percent (u16-only, no way to take a percentage of a signed quantity), reusing the sign-magnitude technique lerp_i16 already proved safe; distinct from a within_percent_i16 predicate, which would answer yes/no rather than produce a scaled value.
//! tags: percent, scale, of, fraction, proportion, multiply, signed, i16, saturate
fn i16_mag(v: i16) -> u32 {
    if v < 0i16 { (0u16.wrapping_sub(v as u16)) as u32 } else { v as u16 as u32 }
}
fn i16_neg(v: i16) -> u16 {
    if v < 0i16 { 1u16 } else { 0u16 }
}
fn run(value: i16, pct: u16) -> i16 {
    let mag = i16_mag(value);
    let neg = i16_neg(value);

    // mag <= 32768 and pct <= 65535, so this product always fits u32 (max ~2.15e9).
    let scaled = mag * (pct as u32) / 100u32;

    // i16 holds magnitudes 0..=32768, but 32768 is only valid when negative (i16::MIN).
    let cap = if neg == 1u16 { 32768u32 } else { 32767u32 };
    let sat = if scaled > cap { cap } else { scaled };

    if neg == 1u16 {
        (0u16.wrapping_sub(sat as u16)) as i16
    } else {
        sat as u16 as i16
    }
}
