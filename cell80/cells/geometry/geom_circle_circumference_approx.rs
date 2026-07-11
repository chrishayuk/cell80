//! Circumference of a circle with integer radius r, floor(2*pi*r) via the same Q8.8 fixed-point pi constant as geom_circle_area_approx's squared-radius formula: (r*1608)>>8, where 1608 = 2*804 -- the simpler non-squaring sibling of the same README-flagged gap (no cell anywhere touches a circle's circumference or area).
//! tags: geometry, circle, circumference, perimeter, pi, fixed-point, approx, radius
//! limits: escalates (halt 0xFF05, needs_wider_math) if the shifted result would exceed u16::MAX (r >= 10434)
fn run(r: u16) -> u16 {
    let rw = r as u32;
    let scaled = rw * 1608u32;
    let result = scaled >> 8u32;
    if result > 65535u32 { halt(0xFF05u16); }
    result as u16
}
