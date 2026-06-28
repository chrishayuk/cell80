//! Returns 1 if lo < x < hi (strictly inside, exclusive bounds), else 0.
//! tags: bounds, between, exclusive, strictly, interval, open
fn run(x: u16, lo: u16, hi: u16) -> u16 { ((lo < x) && (x < hi)) as u16 }
