//! Median (middle value) of three.
//! tags: median, middle, three, stat, midpoint, central
fn run(a: u16, b: u16, c: u16) -> u16 {
    let mut lo = a; let mut hi = a;
    if b < lo { lo = b; } if c < lo { lo = c; }
    if b > hi { hi = b; } if c > hi { hi = c; }
    a.wrapping_add(b).wrapping_add(c).wrapping_sub(lo).wrapping_sub(hi)
}
