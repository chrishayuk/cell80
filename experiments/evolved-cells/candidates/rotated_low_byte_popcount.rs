//! Population count of the low byte of x after rotating its bits left by 4.
//! tags: bits, popcount, rotate, byte, count, ones
fn run(x: u16) -> u16 {
    let s_0 = (12u16) & 15u16 ;
    let out0 = ((x) << s_0) | ((x) >> (16u16 - s_0 & 15u16));
    
    let out1 = (out0) >> 8u16;
    let mut v_2 = (out1) ; let mut c_2 = 0u16 ; while v_2 != 0u16 { c_2 = c_2 + (v_2 & 1u16) ; v_2 = v_2 >> 1u16 ; }
    let out2 = c_2;
    out2
}
