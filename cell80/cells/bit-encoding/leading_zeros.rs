//! Count of leading (high) zero bits in the 16-bit value (16 for x == 0).
//! tags: bits, leading-zeros, clz, count, high, zeros
fn run(x: u16) -> u16 {
    let mut v = x; let mut c = 0u16;
    while c < 16u16 && (v & 0x8000u16) == 0u16 { v = v << 1u16; c = c + 1u16; }
    c
}
