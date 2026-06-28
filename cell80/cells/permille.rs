//! Per-mille (parts per thousand): part*1000/whole (0 if whole == 0).
//! tags: permille, thousandths, ratio, proportion, rate, per-thousand
fn run(part: u16, whole: u16) -> u16 { let mut r = 0u16; if whole != 0u16 { r = part * 1000u16 / whole; } r }
