//! Clamp a signed value to the inclusive range [lo, hi] — the signed counterpart of clamp (which only works over u16). Also the exact form of "hard tanh" in Q8.8 fixed point (clamp_i16(x, -256, 256)): tanh_hard(x) = 2*sigmoid_hard(2x)-1 reduces algebraically to clamp(x, -1, 1), so q_tanh was deliberately not shipped as a second cell — same formula, different name, exactly the case the admission gate exists to catch.
//! tags: clamp, signed, i16, bounds, range, delta, tanh, hardtanh, activation, q8.8, fixed-point
fn run(x: i16, lo: i16, hi: i16) -> i16 {
    if x < lo {
        lo
    } else if x > hi {
        hi
    } else {
        x
    }
}
