//! Percentage of a whole: part*100/whole, in 0..100+ (0 if whole == 0).
//! tags: percent, percentage, ratio, proportion, fraction, rate
fn run(part: u16, whole: u16) -> u16 { let mut r = 0u16; if whole != 0u16 { r = part * 100u16 / whole; } r }
