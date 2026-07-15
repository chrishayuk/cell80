//! Raw wrapping subtract: a - b, wrapping past 0 instead of saturating or checking -- unlike sub_sat (floors at 0) or sub_checked_u32 (escalates), this never traps or clamps.
//! tags: math, arithmetic, subtract, minus, wrapping, raw, unchecked, underflow
fn run(a: u16, b: u16) -> u16 { a.wrapping_sub(b) }
