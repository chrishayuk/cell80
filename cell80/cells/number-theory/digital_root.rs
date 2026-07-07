//! Digital root: repeatedly sum the decimal digits of n until a single digit remains, computed via the exact closed form (1 + (n-1) mod 9, 0 for n == 0) rather than iterating -- distinct from digit_sum (one summing pass) and persistent_digital_root (which counts the iterations this cell short-circuits).
//! tags: number, digital-root, digit, sum, iterate, single-digit, math
fn run(n: u16) -> u16 {
    if n == 0u16 { 0u16 } else { 1u16 + (n - 1u16) % 9u16 }
}
