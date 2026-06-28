//! Index (0 or 1) of the smaller of two values; ties → 0.
//! tags: argmin, index, which, smaller, choose, select
fn run(a: u16, b: u16) -> u16 { (b < a) as u16 }
