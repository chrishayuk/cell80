//! Mode of three values: the value that repeats (ties/all-distinct → the first, a).
//! tags: mode, most-common, repeated, three, stat, majority
fn run(a: u16, b: u16, c: u16) -> u16 { if a == b { a } else if a == c { a } else if b == c { b } else { a } }
