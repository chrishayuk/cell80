//! Rescale x within [lo, hi] to a 0..100 percentage (clamped; 0 if hi <= lo).
//! tags: normalize, rescale, scale, percent, map-range, proportion
fn run(x: u16, lo: u16, hi: u16) -> u16 {
    let mut r = 0u16;
    if hi > lo {
        let mut c = x;
        if c < lo { c = lo; }
        if c > hi { c = hi; }
        r = (c - lo) * 100u16 / (hi - lo);
    }
    r
}
