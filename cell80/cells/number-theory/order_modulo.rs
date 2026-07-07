//! Multiplicative order of a mod n: the smallest k >= 1 with a^k == 1 (mod n). Requires gcd(a, n) == 1 (else no finite order exists) -- the order always divides euler_totient(n), so the search loop is bounded by n itself.
//! tags: number, order, multiplicative, modular, modulo, group, math
//! limits: escalates (halt 0xFF06, out_of_domain) if n < 2 or gcd(a, n) != 1
fn run(a: u16, n: u16) -> u16 {
    if n < 2u16 { halt(0xFF06u16); }
    let a0 = a % n;
    if gcd(a0, n) != 1u16 { halt(0xFF06u16); }
    let mut cur = a0;
    let mut k = 1u16;
    while cur != 1u16 {
        let prod = cur as u32 * a0 as u32;
        cur = (prod % n as u32) as u16;
        k = k + 1u16;
    }
    k
}
