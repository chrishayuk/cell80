//! Returns 1 if n is a perfect square, else 0.
//! tags: number, square, perfect-square, predicate, sqrt, root
fn run(n: u16) -> u16 { let r = isqrt(n); (r * r == n) as u16 }
