//! Spread of three values: max − min.
//! tags: range, spread, span, stat, three, extent
fn run(a: u16, b: u16, c: u16) -> u16 {
    let mut lo = a; let mut hi = a;
    if b < lo { lo = b; } if c < lo { lo = c; }
    if b > hi { hi = b; } if c > hi { hi = c; }
    hi - lo
}
