// flags bit 0 = old presynaptic spike; bit 1 = new postsynaptic spike.
// The current lowerer drops signedness on 16-bit casts/local annotations.
// A typed helper parameter retains the signed division semantics.
fn bounded_delta(a: i16) -> i16 {
    let mut d: i16 = a / 64i16;
    if d > 1i16 { d = 1i16; }
    if d < -1i16 { d = -1i16; }
    d
}
fn run(pre_trace: u16, post_trace: u16, flags: u16) -> i16 {
    bounded_delta((pre_trace * ((flags >> 1u16) & 1u16)) as i16
          - (post_trace * (flags & 1u16)) as i16)
}
