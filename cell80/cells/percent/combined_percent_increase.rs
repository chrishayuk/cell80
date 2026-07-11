//! Combine two successive percent increases pct_a then pct_b into one equivalent single percent rate: pct_a + pct_b + pct_a*pct_b/100, computed in u32 internally and saturating at 65535 -- distinct from compound_increase_by_bps, which loops the SAME rate N times, this composes two DIFFERENT rates once in closed form.
//! tags: percent, increase, combine, successive, compose, stack, two-step, rate, markup
fn run(pct_a: u16, pct_b: u16) -> u16 {
    let a = pct_a as u32;
    let b = pct_b as u32;
    let cross = a * b / 100u32;
    let total = a + b + cross;
    if total > 65535u32 { 65535u16 } else { total as u16 }
}
