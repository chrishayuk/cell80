//! The Jacobi symbol (a/n) for odd n > 0: 1 if a is a quadratic residue mod every prime factor of n (with multiplicity) an even number of times, -1 for an odd number of times, 0 if gcd(a, n) > 1. Computed by the standard law-of-quadratic-reciprocity reduction, tracking the sign as a parity flip (XOR) rather than a signed accumulator, since every intermediate value stays a plain nonnegative u16.
//! tags: number, jacobi, symbol, quadratic, residue, reciprocity, legendre, modular, math
//! limits: escalates (halt 0xFF06, out_of_domain) if n == 0 or n is even
fn run(a: u16, n: u16) -> i16 {
    if n == 0u16 { halt(0xFF06u16); }
    if n % 2u16 == 0u16 { halt(0xFF06u16); }
    let mut aa = a % n;
    let mut nn = n;
    let mut flip = 0u16;
    while aa != 0u16 {
        while aa % 2u16 == 0u16 {
            aa = aa / 2u16;
            let r = nn % 8u16;
            if r == 3u16 || r == 5u16 { flip = flip ^ 1u16; }
        }
        let tmp = aa;
        aa = nn;
        nn = tmp;
        if aa % 4u16 == 3u16 && nn % 4u16 == 3u16 { flip = flip ^ 1u16; }
        aa = aa % nn;
    }
    if nn == 1u16 {
        if flip == 0u16 { 1i16 } else { -1i16 }
    } else {
        0i16
    }
}
