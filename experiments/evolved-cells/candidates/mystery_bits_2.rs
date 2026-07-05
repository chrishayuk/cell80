//! Popcount of a 6-step OR/rotate/AND/rotate/XOR mask chain over x.
//! tags: bits, popcount, mask, rotate, xor, experimental
fn run(x: u16) -> u16 {
    
    let out0 = (x) | (3855u16);
    
    let out1 = (out0) & (43690u16);
    let mut v_2 = (out1) ; let mut c_2 = 0u16 ; while v_2 != 0u16 { c_2 = c_2 + (v_2 & 1u16) ; v_2 = v_2 >> 1u16 ; }
    let out2 = c_2;
    out2
}
