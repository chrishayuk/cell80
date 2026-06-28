//! Modular exponentiation: (base^exp) mod m (0 if m == 0). u16 domain m <= 256.
//! tags: number, modular, exponent, pow-mod, modulo, crypto
fn run(base: u16, exp: u16, m: u16) -> u16 {
    let mut r = 0u16;
    if m != 0u16 {
        r = 1u16 % m;
        let mut b = base % m;
        let mut e = exp;
        while e != 0u16 {
            if e % 2u16 == 1u16 { r = r * b % m; }
            b = b * b % m;
            e = e / 2u16;
        }
    }
    r
}
