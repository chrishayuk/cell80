//! Midrange of three values: (min + max) / 2.
//! tags: midrange, mid, average, three, stat, center
fn run(a: u16, b: u16, c: u16) -> u16 {
    let lo = imin(imin(a, b), c);
    let hi = imax(imax(a, b), c);
    (lo & hi) + ((lo ^ hi) >> 1u16)
}
