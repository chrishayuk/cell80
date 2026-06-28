//! Smallest power of two >= n (0 if it would exceed 65535; next_pow2(0) = 1).
//! tags: number, power, round-up, pow2, ceiling, bits
fn run(n: u16) -> u16 { let mut p = 1u16; while p < n && p != 0u16 { p = p << 1u16; } p }
