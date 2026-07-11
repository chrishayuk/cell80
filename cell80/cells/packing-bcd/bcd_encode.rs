//! Encode a two-digit decimal value (0-99) as packed BCD: tens in the high nibble, units in the low nibble.
//! tags: bcd, decimal, pack, encode, binary-coded-decimal, two-digit, representation, digits
//! limits: n must be 0-99 (a nibble can't hold a tens digit above 9)
fn run(n: u16) -> u16 {
    ((n / 10u16) << 4u16) | (n % 10u16)
}
