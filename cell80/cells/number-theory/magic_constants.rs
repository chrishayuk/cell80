//! The magic constant of an n x n magic square: M(n) = n*(n^2+1)/2, the sum every row/column/diagonal must total when the square is filled with 1..n^2 — a distinct cubic-growth closed form from triangular's quadratic n*(n+1)/2 and polygonal_number's s-gonal family (different formula, different combinatorial meaning, not a duplicate of either).
//! tags: number, magic, magic-square, magic-constant, sequence, math, combinatorics
//! limits: escalates (halt 0xFF05, needs_wider_math) if M(n) would exceed 65535
fn run(n: u16) -> u16 {
    let nw = n as u32;
    let sq = mul_checked_u32(nw, nw);
    let sq1 = add_checked_u32(sq, 1u32);
    let total = if n % 2u16 == 0u16 {
        mul_checked_u32(nw / 2u32, sq1)
    } else {
        mul_checked_u32(nw, sq1 / 2u32)
    };
    if total > 65535u32 { halt(0xFF05u16); }
    total as u16
}
