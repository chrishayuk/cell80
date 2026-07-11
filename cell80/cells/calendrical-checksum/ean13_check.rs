//! Validates a full 13-digit EAN-13/UPC-A/ISBN-13 barcode via mod-10 alternating-weight checksum (weight 1 then weight 3 alternating from the rightmost digit) over hi/lo u32 halves (7+6 decimal digits, hi the more significant half) spanning all 13 digits including the check digit -- structurally distinct from luhn_check (mod-10 but doubles every second digit, not triple-weights) and from a mod-11 isbn10-style check, and split differently from ean13_check_digit's 6+6 halves of the 12-digit prefix that cell generates rather than verifies the 13th digit from.
//! tags: checksum, ean13, upc, isbn13, barcode, validation, mod10, wide, u32
//! entry: Ean13Check::run
fn weighted_digit_sum(n: u32) -> u32 {
    let mut x = n;
    let mut pos: u32 = 0u32;
    let mut sum: u32 = 0u32;
    while x != 0u32 {
        let digit = x % 10u32;
        let mut d = digit;
        if pos % 2u32 == 1u32 {
            d = d * 3u32;
        }
        sum = sum + d;
        x = x / 10u32;
        pos = pos + 1u32;
    }
    sum
}
struct Ean13Check { hi: u32, lo: u32, valid: u16 }
impl Ean13Check {
    fn run(&mut self) -> u16 {
        let sum_hi = weighted_digit_sum(self.hi);
        let sum_lo = weighted_digit_sum(self.lo);
        let total = sum_hi + sum_lo;
        let v = (total % 10u32 == 0u32) as u16;
        self.valid = v;
        v
    }
}
