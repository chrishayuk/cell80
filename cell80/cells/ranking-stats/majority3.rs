//! Returns 1 if at least two of three values are equal, else 0.
//! tags: majority, vote, consensus, agree, three, predicate
fn run(a: u16, b: u16, c: u16) -> u16 { ((a == b) || (b == c) || (a == c)) as u16 }
