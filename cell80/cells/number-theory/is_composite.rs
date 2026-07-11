//! Returns 1 if n is composite (n > 1 and not prime), else 0 -- folds the n>1 edge case in directly rather than negating is_prime (0 and 1 are neither prime nor composite).
//! tags: number, composite, prime, primality, predicate, factor, divisor
fn run(n: u16) -> u16 {
    let mut r = 0u16;
    if n > 1u16 {
        let mut is_prime_flag = 1u16;
        let mut d = 2u16;
        // Same bound as is_prime: a composite u16 always has a factor <= 255
        // (257*257 > 65535), and `d < 256` keeps `d * d` from wrapping.
        while d < 256u16 && d * d <= n { if n % d == 0u16 { is_prime_flag = 0u16; } d = d + 1u16; }
        r = (is_prime_flag == 0u16) as u16;
    }
    r
}
