//! Spread of three values: max − min.
//! tags: range, spread, span, stat, three, extent
fn run(a: u16, b: u16, c: u16) -> u16 { imax(imax(a, b), c) - imin(imin(a, b), c) }
