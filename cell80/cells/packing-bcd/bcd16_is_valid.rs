//! Predicate: returns 1 if all four nibbles of a packed 4-digit BCD u16 (bcd_decode16/bcd_encode16's format) are valid decimal digits 0-9, else 0 -- the 4-nibble extension of bcd_is_valid, mirroring the bcd_encode/bcd_encode16 2-digit/4-digit ladder.
//! tags: bcd, decimal, validate, predicate, binary-coded-decimal, nibble, u16, four-digit
fn run(bcd: u16) -> u16 {
    let thousands = (bcd >> 12u16) & 0xFu16;
    let hundreds = (bcd >> 8u16) & 0xFu16;
    let tens = (bcd >> 4u16) & 0xFu16;
    let units = bcd & 0xFu16;
    ((thousands <= 9u16) && (hundreds <= 9u16) && (tens <= 9u16) && (units <= 9u16)) as u16
}
