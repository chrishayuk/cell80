//! Round-to-nearest integer division a/b, ties rounding up (same tie convention as round_to_multiple); 0 if b == 0. Distinct from the pack's ceil_div (always rounds up) and safe_div (always truncates/floors) -- this rounds to the CLOSEST quotient.
//! tags: math, arithmetic, divide, division, quotient, round, round-nearest, nearest, ties-up, rounding
fn run(a: u16, b: u16) -> u16 {
    if b != 0u16 {
        let q = a / b;
        let r = a % b;
        if r >= b - r { q + 1u16 } else { q }
    } else {
        0u16
    }
}
