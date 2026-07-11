//! Returns 1 if multiplying a numerator-dimension quantity by dimension b is dimensionally defined (same rule table as unit_mul), else 0 — a non-escalating probe for a caller (e.g. a plan verifier) trying several candidate unit pairs without halting.
//! tags: unit, units, dimension, multiply, cancel, predicate, checked
fn run(a: u16, b: u16) -> u16 {
    if a > 9u16 || b > 9u16 { return 0u16; }
    if a == 0u16 && b == 0u16 { return 1u16; }
    if (a == 0u16 && b == 1u16) || (a == 1u16 && b == 0u16) { return 1u16; }
    if a == 3u16 && b == 3u16 { return 1u16; }
    if (a == 4u16 && b == 3u16) || (a == 3u16 && b == 4u16) { return 1u16; }
    if (a == 6u16 && b == 0u16) || (a == 0u16 && b == 6u16) { return 1u16; }
    if (a == 7u16 && b == 2u16) || (a == 2u16 && b == 7u16) { return 1u16; }
    if (a == 8u16 && b == 2u16) || (a == 2u16 && b == 8u16) { return 1u16; }
    if (a == 9u16 && b == 2u16) || (a == 2u16 && b == 9u16) { return 1u16; }
    0u16
}
