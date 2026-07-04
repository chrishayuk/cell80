//! nth triangular number: 1+2+...+n = n*(n+1)/2 (overflow-safe; u16 domain n <= 361).
//! tags: number, triangular, sum, series, math, sequence
fn run(n: u16) -> u16 { if n % 2u16 == 0u16 { (n / 2u16) * (n + 1u16) } else { n * ((n + 1u16) / 2u16) } }
