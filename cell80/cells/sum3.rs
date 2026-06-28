//! Sum of three values (saturating at 65535).
//! tags: sum, add, total, three, aggregate, accumulate
fn run(a: u16, b: u16, c: u16) -> u16 {
    let s1 = a.wrapping_add(b);
    let mut r = s1; if s1 < a { r = 65535u16; }
    let s2 = r.wrapping_add(c);
    if s2 < r { r = 65535u16; } else { r = s2; }
    r
}
