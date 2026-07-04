//! Index (0, 1, or 2) of the smallest of three values; ties → lowest index.
//! tags: argmin, index, which, smallest, choose, select
fn run(a: u16, b: u16, c: u16) -> u16 { if b < a { if c < b { 2u16 } else { 1u16 } } else if c < a { 2u16 } else { 0u16 } }
