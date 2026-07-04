//! Clamp a signed value to the inclusive range [lo, hi] — the signed counterpart of clamp (which only works over u16).
//! tags: clamp, signed, i16, bounds, range, delta
fn run(x: i16, lo: i16, hi: i16) -> i16 {
    if x < lo {
        lo
    } else if x > hi {
        hi
    } else {
        x
    }
}
