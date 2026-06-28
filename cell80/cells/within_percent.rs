//! Returns 1 if actual is within pct percent of target (|actual-target|*100 <= target*pct).
//! tags: percent, within, tolerance, close, near, approx
fn run(actual: u16, target: u16, pct: u16) -> u16 {
    let mut d = 0u16;
    if actual > target { d = actual - target; } else { d = target - actual; }
    (d * 100u16 <= target * pct) as u16
}
