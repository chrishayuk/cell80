//! Encode a four-digit decimal value (0-9999) as packed BCD across a full u16: thousands, hundreds, tens, units, one decimal digit per nibble -- the 4-nibble extension of bcd_encode's 2-nibble (0-99) form, parallel to how pack_u8's byte ladder extends to pack_u16_pair's word ladder.
//! tags: bcd, decimal, pack, encode, binary-coded-decimal, wide, nibble
//! limits: n must be 0-9999 (a nibble can't hold a digit above 9)
fn run(n: u16) -> u16 {
    let thousands = n / 1000u16;
    let hundreds = (n / 100u16) % 10u16;
    let tens = (n / 10u16) % 10u16;
    let units = n % 10u16;
    (thousands << 12u16) | (hundreds << 8u16) | (tens << 4u16) | units
}
