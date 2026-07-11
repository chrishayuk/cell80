//! Stateless symmetric deadband (dead zone) around a center setpoint: returns 0 when the offset |value - center| is within band_width (inclusive), else passes that offset (value - center) through unchanged -- a pure per-call formula with no persisted field and no memory of any prior call, unlike hysteresis (a stateful two-threshold Schmitt-trigger latch that remembers which side it last settled on and needs separate high/low thresholds rather than one band around a single center).
//! tags: deadband, dead-zone, dead-band, setpoint, band, tolerance, offset, error-signal, control, control-systems, stateless, signed, i16
//! entry: run
//! limits: escalates (halt 0xFF05, needs_wider_math) if the true signed offset value - center does not fit back in i16 (e.g. value=i16::MAX, center=i16::MIN)
fn i16_mag(v: i16) -> u32 {
    if v < 0i16 { (0u16.wrapping_sub(v as u16)) as u32 } else { v as u16 as u32 }
}
fn i16_neg(v: i16) -> u16 {
    if v < 0i16 { 1u16 } else { 0u16 }
}
fn run(value: i16, center: i16, band_width: u16) -> i16 {
    let value_mag = i16_mag(value);
    let value_neg = i16_neg(value);
    let center_mag = i16_mag(center);
    // sign of -center: flip center's sign flag, never negate center itself (sub_i16's own shape).
    let center_neg_f = 1u16 - i16_neg(center);

    // diff = value - center = value + (-center), the smag_add shape sub_i16/lerp_i16 use.
    let mut diff_mag = 0u32;
    let mut diff_neg = 0u16;
    if value_neg == center_neg_f {
        diff_mag = add_checked_u32(value_mag, center_mag);
        diff_neg = value_neg;
    } else if value_mag >= center_mag {
        diff_mag = value_mag - center_mag;
        diff_neg = if diff_mag == 0u32 { 0u16 } else { value_neg };
    } else {
        diff_mag = center_mag - value_mag;
        diff_neg = center_neg_f;
    }

    if diff_mag <= band_width as u32 {
        return 0i16;
    }

    // Outside the band: escalate if the true signed offset doesn't fit back in i16.
    if diff_mag > 32768u32 { halt(0xFF05u16); }
    if diff_mag == 32768u32 && diff_neg == 0u16 { halt(0xFF05u16); }

    if diff_neg == 1u16 {
        (0u16.wrapping_sub(diff_mag as u16)) as i16
    } else {
        diff_mag as u16 as i16
    }
}
