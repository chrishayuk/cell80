//! Rescale x within [lo, hi] to a 0..100 percentage (clamped; 0 if hi <= lo).
//! tags: normalize, rescale, scale, percent, map-range, proportion
fn run(x: u16, lo: u16, hi: u16) -> u16 {
    if hi > lo { let c = clamp_to(x, lo, hi); (c - lo) * 100u16 / (hi - lo) } else { 0u16 }
}
