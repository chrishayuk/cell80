//! Smallest prime factor of n (n >= 2) — the least prime p dividing n; returns n itself if n is prime.
//! tags: number, prime, factor, smallest, divisor, factorization, number-theory
//! limits: escalates (halt 0xFF06, out_of_domain) if n < 2
fn run(n: u16) -> u16 {
    if n < 2u16 { halt(0xFF06u16); }
    let mut d = 2u16;
    let mut found = 0u16;
    while d < 256u16 && d * d <= n && found == 0u16 {
        if n % d == 0u16 { found = d; }
        d = d + 1u16;
    }
    if found != 0u16 { found } else { n }
}
