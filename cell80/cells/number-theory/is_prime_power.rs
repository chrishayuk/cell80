//! Returns 1 if n (n >= 2) is a prime power p^k for some prime p and k >= 1, else 0 -- finds n's smallest prime factor p by inline trial division, then strips every factor of p out of n; n was built from p alone iff that leaves exactly 1, a check smallest_prime_factor alone cannot make (it names p but never confirms no other prime remains).
//! tags: number, prime, power, prime-power, factorization, predicate, number-theory
fn run(n: u16) -> u16 {
    if n < 2u16 { return 0u16; }
    let mut d = 2u16;
    let mut found = 0u16;
    while d < 256u16 && d * d <= n && found == 0u16 {
        if n % d == 0u16 { found = d; }
        d = d + 1u16;
    }
    let p = if found != 0u16 { found } else { n };
    let mut m = n;
    while m % p == 0u16 { m = m / p; }
    (m == 1u16) as u16
}
