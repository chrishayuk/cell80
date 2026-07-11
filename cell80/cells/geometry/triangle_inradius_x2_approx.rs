//! Twice the inradius of a triangle with integer sides (a, b, c): floor(floor(4*Area)/(a+b+c)), reusing heron_16a2's exact 16*Area^2 rearrangement and triangle_area_x4_approx's own inline isqrt loop to get floor(4*Area), then dividing by the perimeter since 2r = 4*Area/(a+b+c) -- the pack's first triangle metric that is a length derived from area/perimeter rather than an area itself.
//! tags: geometry, triangle, inradius, radius, length, heron, sqrt, isqrt, wide, u32, checked, escalate
//! limits: escalates (halt 0xFF06, out_of_domain) if a, b, c do not form a valid (non-degenerate) triangle; escalates (halt 0xFF05, needs_wider_math) if either factor-pair product overflows u32
fn run(a: u16, b: u16, c: u16) -> u16 {
    let aw = a as u32;
    let bw = b as u32;
    let cw = c as u32;
    if aw + bw <= cw || bw + cw <= aw || aw + cw <= bw { halt(0xFF06u16); }
    let s1 = aw + bw + cw;
    let s2 = bw + cw - aw;
    let s3 = aw + cw - bw;
    let s4 = aw + bw - cw;
    let p1 = mul_checked_u32(s1, s2);
    let p2 = mul_checked_u32(s3, s4);
    let sixteen_a2 = mul_checked_u32(p1, p2);

    // Branch-free bitwise integer square root of sixteen_a2 (the same loop isqrt_u32 runs).
    let mut val = sixteen_a2;
    let mut res = 0u32;
    let mut bit = 1u32 << 30u32;
    while bit > val { bit = bit >> 2u32; }
    while bit != 0u32 {
        if val >= res + bit {
            val = val - (res + bit);
            res = (res >> 1u32) + bit;
        } else {
            res = res >> 1u32;
        }
        bit = bit >> 2u32;
    }

    // res is now floor(4*Area); s1 is the perimeter (a+b+c), always > 0 for a valid triangle.
    let perimeter = s1;
    (res / perimeter) as u16
}
