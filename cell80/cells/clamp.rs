//! Clamp a value to the inclusive range [lo, hi].
//! tags: math, range, clamp, bound, limit, restrict, constrain, floor, ceiling, minimum, maximum
fn run(x: u16, lo: u16, hi: u16) -> u16 { if x > hi { hi } else if x < lo { lo } else { x } }
