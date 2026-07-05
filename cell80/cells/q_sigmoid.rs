//! Q8.8 fixed-point "hard sigmoid": a well-known piecewise-linear stand-in for the true sigmoid, clamp(x/4 + 0.5, 0, 1) — exact at x=0, saturating to 0/1 outside roughly [-4, 4], monotonic and cheap everywhere between. Input is signed (Q8.8, negative values meaningful, e.g. -256 = -1.0); output is unsigned Q8.8 in [0, 256] (0.0 to 1.0). q_tanh is deliberately not a separate cell: the same derivation (tanh(x) = 2*sigmoid(2x)-1) reduces to clamp_i16(x, -256, 256) exactly, already covered by that cell's own tags.
//! tags: fixed-point, q8.8, sigmoid, activation, piecewise, approximation, hard-sigmoid, signed, i16
fn run(x: i16) -> u16 {
    let scaled = x / 4i16 + 128i16;
    let mut r = scaled;
    if r < 0i16 { r = 0i16; }
    if r > 256i16 { r = 256i16; }
    r as u16
}
