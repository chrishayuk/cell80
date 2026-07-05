//! Popcount of x, OR'd with 0xAAAA, rotated left 8, XOR'd with 0x5555.
//! tags: bits, popcount, mask, rotate, xor, experimental
fn run(x: u16) -> u16 {
    
    let out0 = (x) & (21845u16);
    let s_1 = (4u16) & 15u16 ;
    let out1 = ((out0) << s_1) | ((out0) >> (16u16 - s_1 & 15u16));
    
    let out2 = (out1) ^ (21845u16);
    
    let out3 = (out2) | (43690u16);
    let mut v_4 = (out3) ; let mut c_4 = 0u16 ; while v_4 != 0u16 { c_4 = c_4 + (v_4 & 1u16) ; v_4 = v_4 >> 1u16 ; }
    let out4 = c_4;
    out4
}
