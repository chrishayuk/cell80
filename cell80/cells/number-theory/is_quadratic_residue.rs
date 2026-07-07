//! Check whether x is a quadratic residue mod p: does some y in [0, p) satisfy y*y == x (mod p)? Works for any modulus p >= 2, not just primes, via direct search over every residue -- so cost scales with p (like is_prime_u32; budget a larger --cycles for p much beyond a few thousand).
//! tags: number, quadratic, residue, modular, modulo, square, predicate, math
//! limits: escalates (halt 0xFF06, out_of_domain) if p < 2
fn run(x: u16, p: u16) -> u16 {
    if p < 2u16 { halt(0xFF06u16); }
    let target = x % p;
    let mut y = 0u16;
    let mut found = 0u16;
    while y < p {
        let sq = (y as u32 * y as u32 % p as u32) as u16;
        if sq == target { found = 1u16; }
        y = y + 1u16;
    }
    found
}
