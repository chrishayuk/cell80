//! Popcount of x, OR'd with 0xAAAA, rotated left 8, XOR'd with 0x5555.
//! tags: bits, popcount, mask, rotate, xor, experimental
fn run(x: u16) -> u16 {
    let s_0 = (4u16) & 15u16;
    let out0 = ((x) << s_0) | ((x) >> (16u16 - s_0 & 15u16));
    
    let out1 = (out0) ^ (21845u16);
    
    let out2 = (out1) | (43690u16);
    let mut v_3 = (out2); let mut c_3 = 0u16; while v_3 != 0u16 { c_3 = c_3 + (v_3 & 1u16); v_3 = v_3 >> 1u16; }
    let out3 = c_3;
    out3
}
