//! Index (0, 1, or 2) of the largest of three values; ties → lowest index.
//! tags: argmax, index, which, largest, choose, select
fn run(a: u16, b: u16, c: u16) -> u16 { let mut r = 0u16; let mut m = a; if b > m { m = b; r = 1u16; } if c > m { r = 2u16; } r }
