//! Q8.8 fixed-point triple multiply: chains two q_mul-style widen-shift steps -- step1 = (a*b)>>8, then result = (step1*c)>>8 -- q_mul has no 3-arg sibling despite the arity-2-to-3 generalization already established elsewhere (mul_checked_u32/mul3_checked_u32, add_checked_u32/add3_checked_u32).
//! tags: fixed-point, q8.8, multiply, triple, chain, scale, math, wide
//! scale: 8
fn run(a: u16, b: u16, c: u16) -> u16 {
    let step1 = ((a as u32 * b as u32) >> 8u32) as u16;
    ((step1 as u32 * c as u32) >> 8u32) as u16
}
