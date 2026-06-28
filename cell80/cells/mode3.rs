//! Mode of three values: the value that repeats (ties/all-distinct → the first, a).
//! tags: mode, most-common, repeated, three, stat, majority
fn run(a: u16, b: u16, c: u16) -> u16 {
    let mut r = a;
    if a == b { r = a; } else if a == c { r = a; } else if b == c { r = b; } else { r = a; }
    r
}
