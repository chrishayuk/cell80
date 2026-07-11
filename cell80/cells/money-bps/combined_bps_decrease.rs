//! Compose two sequential bps decreases into one equivalent single bps rate: combined = bps1 + bps2 - bps1*bps2/10000, derived from 1-(1-r1)(1-r2) -- stacking two successive markdowns/discounts into one effective discount rate.
//! tags: money, bps, basis-points, discount, decrease, compose, combine, stack, checked, wide, u32
//! limits: escalates (halt 0xFF06, out_of_domain) if bps1 > 10000 or bps2 > 10000
fn run(bps1: u16, bps2: u16) -> u16 {
    if bps1 > 10000u16 || bps2 > 10000u16 { halt(0xFF06u16); }
    let cross = mul_checked_u32(bps1 as u32, bps2 as u32) / 10000u32;
    ((bps1 as u32 + bps2 as u32) - cross) as u16
}
