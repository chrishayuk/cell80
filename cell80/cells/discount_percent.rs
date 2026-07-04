//! Decrease a value by pct percent: value - value*pct/100 (0 if pct >= 100).
//! tags: percent, discount, decrease, reduce, markdown, off
fn run(value: u16, pct: u16) -> u16 { if pct < 100u16 { value - value * pct / 100u16 } else { 0u16 } }
