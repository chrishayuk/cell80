//! Omega(n): total count of prime factors of n counted with multiplicity (n >= 1; Omega(1) = 0) -- distinct from little_omega (counts distinct primes only) and factor_count (counts divisors, not prime factors).
//! tags: number, omega, prime, multiplicity, factors, factorization, count, number-theory
//! limits: escalates (halt 0xFF06, out_of_domain) if n == 0
fn run(n: u16) -> u16 {
    if n == 0u16 { halt(0xFF06u16); }
    let mut count = 0u16;
    let mut m = n;
    let mut p = 2u16;
    while p < 256u16 && p * p <= m {
        while m % p == 0u16 { count = count + 1u16; m = m / p; }
        p = p + 1u16;
    }
    if m > 1u16 { count = count + 1u16; }
    count
}
