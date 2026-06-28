//! Wrap x into the cycle [0, m): x % m (0 if m == 0).
//! tags: bounds, wrap, modulo, cycle, around, period
fn run(x: u16, m: u16) -> u16 { let mut r = 0u16; if m != 0u16 { r = x % m; } r }
