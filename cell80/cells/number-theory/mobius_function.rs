//! The Mobius function mu(n): 1 if n = 1, 0 if n has a squared prime factor (not squarefree), else (-1)^omega(n) for squarefree n (n >= 1).
//! tags: number, mobius, mu, squarefree, sign, parity, signed, i16, number-theory
//! limits: escalates (halt 0xFF06, out_of_domain) if n == 0
fn run(n: u16) -> i16 {
    if n == 0u16 { halt(0xFF06u16); }
    if n == 1u16 { return 1i16; }
    let mut m = n;
    let mut p = 2u16;
    let mut count = 0u16;
    let mut squareful = 0u16;
    while p < 256u16 && p * p <= m && squareful == 0u16 {
        if m % p == 0u16 {
            m = m / p;
            if m % p == 0u16 { squareful = 1u16; } else { count = count + 1u16; }
        }
        p = p + 1u16;
    }
    if squareful == 1u16 { return 0i16; }
    if m > 1u16 { count = count + 1u16; }
    if count % 2u16 == 0u16 { 1i16 } else { -1i16 }
}
