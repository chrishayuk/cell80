//! Raw wrapping add: a + b, wrapping past 65535 -- unlike add_sat (clamps) or add_checked_u32 (escalates), this never traps or clamps.
//! tags: math, arithmetic, add, sum, wrapping, raw, unchecked, overflow
fn run(a: u16, b: u16) -> u16 { a.wrapping_add(b) }
