//! Saturating cross-multiply-divide: floor(a*b/c) via a widened u32 product, saturated to 65535 on overflow, 0 if c == 0 -- the generic rescale-by-ratio primitive with all three operands free, distinct from percent/permille/ratio_255/scale_percent (which all bake in a fixed denominator) and NOT the same as composing mul_sat then safe_div (mul_sat would saturate the intermediate product before the divide, corrupting the ratio).
//! tags: math, arithmetic, multiply, divide, ratio, rescale, cross-multiply, wide, saturating, safe
fn run(a: u16, b: u16, c: u16) -> u16 {
    if c == 0u16 {
        0u16
    } else {
        let prod = a as u32 * b as u32;
        let q = prod / c as u32;
        if q > 65535u32 { 65535u16 } else { q as u16 }
    }
}
