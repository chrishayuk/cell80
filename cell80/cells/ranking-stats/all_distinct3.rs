//! Returns 1 if a, b, c are pairwise distinct (a != b && a != c && b != c), else 0 -- the exact logical complement of majority3 ('at least two of three are equal').
//! tags: distinct, unique, pairwise, three, predicate, complement, majority
fn run(a: u16, b: u16, c: u16) -> u16 { ((a != b) && (a != c) && (b != c)) as u16 }
