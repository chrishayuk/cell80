//! Check whether n^2 ends with the decimal digits of n itself (e.g. 5^2=25, 6^2=36, 25^2=625, 76^2=5776) -- a classic "self-reproducing" number check. Computed exactly via n*n mod 10^(digit count of n), so no string/digit-array comparison is needed.
//! tags: number, automorphic, self-reproducing, square, digit, predicate, math
fn run(n: u16) -> u16 {
    let mut digits = 1u32;
    let mut t = n;
    while t >= 10u16 {
        digits = digits * 10u32;
        t = t / 10u16;
    }
    let modulus = digits * 10u32;
    let sq = mul_checked_u32(n as u32, n as u32);
    ((sq % modulus) as u16 == n) as u16
}
