//! The Liouville function lambda(n) = (-1)^Omega(n) (n >= 1), tracking the sign as a parity flip (XOR) over big_omega's own prime-factors-with-multiplicity loop -- distinct from mobius_function, which is 0 for any non-squarefree n, whereas lambda is always +-1 and defined for every n.
//! tags: number, liouville, lambda, omega, prime, multiplicity, factors, sign, parity, signed, i16, number-theory
//! limits: escalates (halt 0xFF06, out_of_domain) if n == 0
fn run(n: u16) -> i16 {
    if n == 0u16 { halt(0xFF06u16); }
    let mut m = n;
    let mut p = 2u16;
    let mut flip = 0u16;
    while p < 256u16 && p * p <= m {
        while m % p == 0u16 { flip = flip ^ 1u16; m = m / p; }
        p = p + 1u16;
    }
    if m > 1u16 { flip = flip ^ 1u16; }
    if flip == 0u16 { 1i16 } else { -1i16 }
}
