//! Popcount of a 6-step OR/rotate/AND/rotate/XOR mask chain over x.
//! tags: bits, popcount, mask, rotate, xor, experimental
fn run(x: u16) -> u16 {
    
    let out0 = (x) | (3855u16);
    let s_1 = (2u16) & 15u16;
    let out1 = ((out0) << s_1) | ((out0) >> (16u16 - s_1 & 15u16));
    
    let out2 = (out1) & (43690u16);
    let mut v_3 = (out2); let mut c_3 = 0u16; while v_3 != 0u16 { c_3 = c_3 + (v_3 & 1u16); v_3 = v_3 >> 1u16; }
    let out3 = c_3;
    
    let out4 = (out3) & 0xFFu16;
    out4
}
