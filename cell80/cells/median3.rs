//! Median (middle value) of three.
//! tags: median, middle, three, stat, midpoint, central
fn run(a: u16, b: u16, c: u16) -> u16 {
    let lo = imin(imin(a, b), c);
    let hi = imax(imax(a, b), c);
    a.wrapping_add(b).wrapping_add(c).wrapping_sub(lo).wrapping_sub(hi)
}
