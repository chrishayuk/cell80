//! Sum of three values (saturating at 65535).
//! tags: sum, add, total, three, aggregate, accumulate
fn run(a: u16, b: u16, c: u16) -> u16 {
    let ab = a.wrapping_add(b);
    let t = if ab < a { 65535u16 } else { ab };
    let s = t.wrapping_add(c);
    if s < t { 65535u16 } else { s }
}
