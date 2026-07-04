//! Returns 1 if lo <= x <= hi, else 0.
//! tags: validation, validate, range, bounds, check
fn run(x: u16, lo: u16, hi: u16) -> u16 { (lo <= x && x <= hi) as u16 }
