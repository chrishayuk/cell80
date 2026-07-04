//! Ceiling division: the smallest k with k*b >= a (0 if b == 0). Rounds up.
//! tags: math, arithmetic, divide, ceiling, round-up, quotient
fn run(a: u16, b: u16) -> u16 { if b != 0u16 && a != 0u16 { (a - 1u16) / b + 1u16 } else { 0u16 } }
