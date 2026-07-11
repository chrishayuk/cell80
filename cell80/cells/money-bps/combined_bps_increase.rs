//! Compose two sequential bps increases into one equivalent single bps rate: combined = bps1 + bps2 + bps1*bps2/10000, derived from (1+r1)(1+r2)-1 -- folding a markup and a separately-stated tax into one effective rate, distinct from compound_increase_by_bps which loops the SAME rate N times rather than composing two different rates.
//! tags: money, bps, basis-points, markup, tax, increase, compose, combine, stack, checked, wide, u32
//! limits: escalates (halt 0xFF05, needs_wider_math) if bps1 + bps2 + the cross term would exceed 65535
fn run(bps1: u16, bps2: u16) -> u16 {
    let cross = mul_checked_u32(bps1 as u32, bps2 as u32) / 10000u32;
    let total = add_checked_u32(add_checked_u32(bps1 as u32, bps2 as u32), cross);
    if total > 65535u32 { halt(0xFF05u16); }
    total as u16
}
