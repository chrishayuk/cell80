//! Largest of three values.
//! tags: max, maximum, largest, greatest, extremum, three
fn run(a: u16, b: u16, c: u16) -> u16 { let mut m = a; if b > m { m = b; } if c > m { m = c; } m }
