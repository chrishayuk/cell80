//! Population count of just the high byte of x (low byte ignored).
//! tags: bits, popcount, byte, high, count, ones, bitcount
fn run(x: u16) -> u16 {
    let out0 = (x >> 8u16);
    let mut v_1 = out0; let mut c_1 = 0u16; while v_1 != 0u16 { c_1 = c_1 + (v_1 & 1u16); v_1 = v_1 >> 1u16; } let out1 = c_1;
    out1
}
