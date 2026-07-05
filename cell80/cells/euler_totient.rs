//! Euler's totient (phi): count of integers in [1, n] coprime to n (n >= 1; phi(1) = 1 by convention).
//! tags: number, totient, euler, phi, coprime, count, number-theory
//! limits: escalates (halt 0xFF06, out_of_domain) if n == 0
fn run(n: u16) -> u16 {
    if n == 0u16 { halt(0xFF06u16); }
    let mut result = n;
    let mut m = n;
    let mut p = 2u16;
    while p < 256u16 && p * p <= m {
        if m % p == 0u16 {
            result = result - result / p;
            while m % p == 0u16 { m = m / p; }
        }
        p = p + 1u16;
    }
    if m > 1u16 { result = result - result / m; }
    result
}
