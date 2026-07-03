//! Returns 1 if n is prime, else 0.
//! tags: number, prime, primality, predicate, factor, divisor
fn run(n: u16) -> u16 {
    let mut r = 1u16;
    if n < 2u16 { r = 0u16; }
    let mut d = 2u16;
    // `d < 256` bounds the trial: a composite u16 always has a factor <= 255
    // (257*257 > 65535), and it keeps `d * d` from wrapping — unbounded, the loop
    // never exits for n > 65025 (the factor_count idiom).
    while d < 256u16 && d * d <= n { if n % d == 0u16 { r = 0u16; } d = d + 1u16; }
    r
}
