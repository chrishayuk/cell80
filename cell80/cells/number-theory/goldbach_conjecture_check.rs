//! For even n > 2, returns the smallest prime p (2 <= p <= n/2) such that n-p is also prime, via inline trial-division primality on both p and n-p -- a bounded-witness-search over a *pair* of primality tests, distinct from smallest_prime_factor (single divisor search) and discrete_log_naive/order_modulo (single-value exponent search).
//! tags: number, prime, primality, goldbach, conjecture, search, bounded, pair, sum
//! limits: escalates (halt 0xFF06, out_of_domain) if n is odd or n <= 2
fn is_prime16(x: u16) -> u16 {
    let mut r = 1u16;
    if x < 2u16 { r = 0u16; }
    let mut d = 2u16;
    while d < 256u16 && d * d <= x {
        if x % d == 0u16 { r = 0u16; }
        d = d + 1u16;
    }
    r
}

fn run(n: u16) -> u16 {
    if n <= 2u16 || n % 2u16 != 0u16 { halt(0xFF06u16); }
    let half = n / 2u16;
    let mut p = 2u16;
    let mut found = 0u16;
    while p <= half && found == 0u16 {
        if is_prime16(p) == 1u16 && is_prime16(n - p) == 1u16 {
            found = p;
        }
        p = p + 1u16;
    }
    found
}
