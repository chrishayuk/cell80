//! Arithmetic negation of a signed 16-bit value (-x) -- distinct from abs_i16 (returns an unsigned magnitude, sidestepping the MIN case) and sign_i16 (returns only -1/0/1): this is the only cell that computes -x itself, so it must escalate exactly where a naive negation would silently wrap.
//! tags: negate, negation, signed, i16, delta, sign-flip, invert
//! limits: escalates (halt 0xFF05, needs_wider_math) if x == i16::MIN (-32768), since its negation 32768 has no representation in i16
fn run(x: i16) -> i16 {
    if x == -32768i16 { halt(0xFF05u16); }
    -x
}
