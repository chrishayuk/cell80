//! Returns 1 if lo < x <= hi (half-open interval: open at lo, closed at hi), else 0 — completes the interval family with range_check (fully closed), between_exclusive (fully open), and in_range_closed_open (closed at lo, open at hi).
//! tags: validation, validate, range, bounds, check, half-open, interval
fn run(x: u16, lo: u16, hi: u16) -> u16 { ((lo < x) && (x <= hi)) as u16 }
