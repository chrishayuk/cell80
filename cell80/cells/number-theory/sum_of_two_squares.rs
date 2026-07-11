//! Predicate: does n decompose as a^2 + b^2 for some integers a, b >= 0? Two-pointer search: a from 0 up, b from isqrt(n) down, comparing b*b against n - a*a (never a*a + b*b, which can exceed u16 near n's top end) so it stays O(sqrt(n)) instead of the O(n) an isqrt-per-a scan would cost -- distinct from is_square (single square term) and jacobi_symbol/is_quadratic_residue (multiplicative/modular residue membership, not additive decomposition).
//! tags: number, sum-of-two-squares, additive, decomposition, square, predicate, sqrt, math
fn run(n: u16) -> u16 {
    let mut a = 0u16;
    let mut b = isqrt(n);
    let mut found = 0u16;
    while a <= b && found == 0u16 {
        let rem = n - a * a;
        let sq = b * b;
        if sq == rem {
            found = 1u16;
        } else if sq < rem {
            a = a + 1u16;
        } else {
            b = b - 1u16;
        }
    }
    found
}
