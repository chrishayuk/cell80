//! Apply a signed delta to an unsigned value, clamped to [0, cap] — e.g. a health/resource/score adjustment that can't go negative or exceed a cap (a "risk delta" applied safely).
//! tags: delta, signed, i16, clamp, risk, adjust, health, resource, score, bounds
//! limits: assumes value is already within [0, cap]; clamps the result, not the input
fn run(value: u16, delta: i16, cap: u16) -> u16 {
    if delta >= 0i16 {
        let sum = value.wrapping_add(delta as u16);
        if sum < value || sum > cap { cap } else { sum }
    } else {
        let mag = 0u16.wrapping_sub(delta as u16);
        if mag > value { 0u16 } else { value - mag }
    }
}
