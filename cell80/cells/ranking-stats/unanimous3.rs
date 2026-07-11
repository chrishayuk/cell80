//! Returns 1 if all three values are equal, else 0 -- the strict all-agree sibling of majority3's weaker at-least-two-agree threshold.
//! tags: unanimous, all-equal, consensus, agree, three, predicate, strict
fn run(a: u16, b: u16, c: u16) -> u16 { ((a == b) && (b == c)) as u16 }
