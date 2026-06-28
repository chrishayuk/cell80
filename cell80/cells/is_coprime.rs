//! Returns 1 if a and b are coprime (gcd == 1), else 0.
//! tags: number, coprime, gcd, relatively-prime, predicate, divisor
fn run(a: u16, b: u16) -> u16 { (gcd(a, b) == 1u16) as u16 }
