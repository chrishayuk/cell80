//! Returns 1 if actual is within pct percent of target (|actual-target|*100 <= target*pct).
//! tags: percent, within, tolerance, close, near, approx
fn run(actual: u16, target: u16, pct: u16) -> u16 {
    let d = iabs_diff(actual, target);
    let l = d as u32 * 100u32;
    let r = target as u32 * pct as u32;
    let lh = (l >> 16u32) as u16;
    let rh = (r >> 16u32) as u16;
    (lh < rh || (lh == rh && l as u16 <= r as u16)) as u16
}
