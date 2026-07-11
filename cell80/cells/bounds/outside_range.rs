//! Returns 1 if x is outside the open interval (lo, hi): x <= lo || x >= hi -- the exact logical complement of between_exclusive.
//! tags: bounds, outside, between, exclusive, interval, open, complement
fn run(x: u16, lo: u16, hi: u16) -> u16 { ((x <= lo) || (x >= hi)) as u16 }
