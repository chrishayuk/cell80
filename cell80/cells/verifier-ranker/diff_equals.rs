//! Verifies a claimed difference: returns 1 if a >= b and a - b == remainder, else 0 (including when a < b, since an unsigned difference can't be negative).
//! tags: verify, verifier, equation, difference, subtract, leftover, remainder, check, plan, reverse-equation
fn run(a: u16, b: u16, remainder: u16) -> u16 {
    if a < b { return 0u16; }
    ((a - b) == remainder) as u16
}
