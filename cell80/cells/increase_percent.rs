//! Increase a value by pct percent: value + value*pct/100 (saturating at 65535).
//! tags: percent, increase, markup, raise, grow, surcharge
fn run(value: u16, pct: u16) -> u16 { let add = value * pct / 100u16; let s = value.wrapping_add(add); let mut r = s; if s < value { r = 65535u16; } r }
