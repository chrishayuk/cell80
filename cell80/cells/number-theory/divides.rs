//! Returns 1 if a divides b evenly (b % a == 0, a != 0), else 0.
//! tags: number, divides, divisible, factor, predicate, multiple
fn run(a: u16, b: u16) -> u16 { if a != 0u16 { (b % a == 0u16) as u16 } else { 0u16 } }
