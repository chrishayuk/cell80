//! Verifies a claimed sum: returns 1 if a + b == total, else 0 — computed in a wider internal width so a genuine overflow can't false-positive as a match on the wrapped value.
//! tags: verify, verifier, equation, sum, addition, check, plan, reverse-equation
fn run(a: u16, b: u16, total: u16) -> u16 {
    let s: u32 = (a as u32) + (b as u32);
    (s == (total as u32)) as u16
}
