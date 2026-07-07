//! Index (0, 1, or 2) of the largest of three values; ties → lowest index.
//! tags: argmax, index, which, largest, choose, select
fn run(a: u16, b: u16, c: u16) -> u16 { if b > a { if c > b { 2u16 } else { 1u16 } } else if c > a { 2u16 } else { 0u16 } }
