//! Returns 1 if lo <= x < hi (half-open interval: closed at lo, open at hi), else 0 — the scalar 1D counterpart of point_in_rect's per-axis half-open test, distinct from range_check (fully closed) and between_exclusive (fully open).
//! tags: validation, validate, range, bounds, check, half-open, interval, index, slice
fn run(x: u16, lo: u16, hi: u16) -> u16 { ((lo <= x) && (x < hi)) as u16 }
