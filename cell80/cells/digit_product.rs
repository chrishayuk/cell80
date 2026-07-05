//! Product of the decimal digits of n (0 has product 0, its only digit).
//! tags: number, digits, digit-product, decimal, product, math
fn run(n: u16) -> u16 {
    if n == 0u16 { return 0u16; }
    let mut v = n;
    let mut p = 1u16;
    while v != 0u16 {
        p = p * (v % 10u16);
        v = v / 10u16;
    }
    p
}
