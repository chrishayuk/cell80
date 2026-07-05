//! Digital root of n: repeatedly sum digits until one digit remains.
//! tags: number, digits, digital-root, decimal, reduce, math
fn run(x: u16) -> u16 {
    let mut v_0 = (x) ; let mut s_0 = 0u16 ; while v_0 != 0u16 { s_0 = s_0 + v_0 % 10u16 ; v_0 = v_0 / 10u16 ; }
    let out0 = s_0;
    let mut v_1 = (out0) ; let mut s_1 = 0u16 ; while v_1 != 0u16 { s_1 = s_1 + v_1 % 10u16 ; v_1 = v_1 / 10u16 ; }
    let out1 = s_1;
    let mut v_2 = (out1) ; let mut s_2 = 0u16 ; while v_2 != 0u16 { s_2 = s_2 + v_2 % 10u16 ; v_2 = v_2 / 10u16 ; }
    let out2 = s_2;
    out2
}
