//! Index (0 or 1) of the larger of two values; ties → 0.
//! tags: argmax, index, which, larger, choose, select
fn run(a: u16, b: u16) -> u16 { (b > a) as u16 }
