//! Returns 1 if dividing a numerator-unit quantity by a denominator-unit quantity is dimensionally defined (same rule table as unit_div), else 0 — a non-escalating probe for a caller (e.g. a plan verifier) trying several candidate unit pairs without halting.
//! tags: unit, units, dimension, divide, cancel, predicate, checked
fn run(a: u16, b: u16) -> u16 {
    if a > 7u16 || b > 7u16 { return 0u16; }
    if a == b { return 1u16; }
    if a == 1u16 && b == 0u16 { return 1u16; }
    if a == 3u16 && b == 2u16 { return 1u16; }
    if a == 4u16 && b == 3u16 { return 1u16; }
    if a == 5u16 && b == 3u16 { return 1u16; }
    if a == 5u16 && b == 4u16 { return 1u16; }
    0u16
}
