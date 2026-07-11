//! Inverse of normalize_0_100: given range [lo, hi] and percentage pct (0..100, clamped if over), returns the value at that percentage into the range: lo + (hi-lo)*pct/100 (returns lo if hi <= lo).
//! tags: percent, value, range, interpolate, denormalize, inverse, unscale, lerp, map-range, unnormalize
fn run(lo: u16, hi: u16, pct: u16) -> u16 {
    if hi <= lo {
        lo
    } else {
        let p = if pct > 100u16 { 100u16 } else { pct };
        let q = (hi - lo) as u32 * p as u32 / 100u32;
        lo + q as u16
    }
}
