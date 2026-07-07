//! omega(n): count of distinct prime factors of n (n >= 1; omega(1) = 0 by convention) -- distinct from factor_count (counts divisors, not prime factors) and big_omega (counts prime factors with multiplicity, not distinct primes).
//! tags: number, omega, prime, distinct, factors, factorization, count, number-theory
//! limits: escalates (halt 0xFF06, out_of_domain) if n == 0
fn run(n: u16) -> u16 {
    if n == 0u16 { halt(0xFF06u16); }
    let mut count = 0u16;
    let mut m = n;
    let mut p = 2u16;
    while p < 256u16 && p * p <= m {
        if m % p == 0u16 {
            count = count + 1u16;
            while m % p == 0u16 { m = m / p; }
        }
        p = p + 1u16;
    }
    if m > 1u16 { count = count + 1u16; }
    count
}
