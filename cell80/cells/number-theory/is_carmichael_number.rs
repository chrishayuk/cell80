//! Returns 1 if n is a Carmichael number by Korselt's criterion (n composite, squarefree, and (p-1) divides (n-1) for every prime factor p), else 0 -- distinct from smallest_prime_factor/factor_count (no Korselt check) and mobius_function (stops at squarefree-ness, never checks compositeness or divisibility).
//! tags: number, carmichael, korselt, composite, squarefree, pseudoprime, predicate, factorization, number-theory
fn run(n: u16) -> u16 {
    if n < 2u16 { return 0u16; }
    let mut m = n;
    let mut p = 2u16;
    let mut distinct = 0u16;
    let mut squareful = 0u16;
    let mut korselt_ok = 1u16;
    while p < 256u16 && p * p <= m && squareful == 0u16 {
        if m % p == 0u16 {
            m = m / p;
            if m % p == 0u16 {
                squareful = 1u16;
            } else {
                distinct = distinct + 1u16;
                if (n - 1u16) % (p - 1u16) != 0u16 { korselt_ok = 0u16; }
            }
        }
        p = p + 1u16;
    }
    if squareful == 1u16 { return 0u16; }
    if m > 1u16 {
        distinct = distinct + 1u16;
        if (n - 1u16) % (m - 1u16) != 0u16 { korselt_ok = 0u16; }
    }
    ((distinct >= 2u16) && (korselt_ok == 1u16)) as u16
}
