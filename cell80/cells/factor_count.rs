//! Number of positive divisors of n (0 for n == 0).
//! tags: number, divisors, factors, count, tau, number-theory
fn run(n: u16) -> u16 {
    let mut c = 0u16;
    if n != 0u16 {
        let mut d = 1u16;
        while d < 256u16 && d * d < n { if n % d == 0u16 { c = c + 2u16; } d = d + 1u16; }
        if d < 256u16 && d * d == n { c = c + 1u16; }
    }
    c
}
