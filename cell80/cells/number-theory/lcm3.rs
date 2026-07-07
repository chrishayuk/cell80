//! Least common multiple of three values.
//! tags: number, lcm, multiple, common, three
fn run(a: u16, b: u16, c: u16) -> u16 {
    let g1 = gcd(a, b);
    let ab = if g1 != 0u16 { a / g1 * b } else { 0u16 };
    let g2 = gcd(ab, c);
    if g2 != 0u16 { ab / g2 * c } else { 0u16 }
}
