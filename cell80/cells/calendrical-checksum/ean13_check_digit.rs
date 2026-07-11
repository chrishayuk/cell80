//! Computes the 13th EAN-13 check digit from the first 12 digits (split hi/lo, 6 decimal digits each, hi the more significant half): weight-1 digits at odd left-to-right positions (1st, 3rd, ... 11th) plus weight-3 digits at even positions (2nd, 4th, ... 12th), then (10 - sum mod 10) mod 10 -- the generate-side counterpart to ean13_check, needing a u32 state-cell split since 12 digits cannot fit one 16-bit free-fn parameter.
//! tags: checksum, ean13, upc, barcode, check-digit, generate, mod10, wide, u32
//! entry: Ean13CheckDigit::run
fn digit_weighted_sum(n: u32) -> u32 {
    let mut x = n;
    let mut pos: u32 = 0u32;
    let mut sum: u32 = 0u32;
    while x != 0u32 {
        let digit = x % 10u32;
        let mut d = digit;
        if pos % 2u32 == 0u32 {
            d = d * 3u32;
        }
        sum = sum + d;
        x = x / 10u32;
        pos = pos + 1u32;
    }
    sum
}
struct Ean13CheckDigit { hi: u32, lo: u32, digit: u16 }
impl Ean13CheckDigit {
    fn run(&mut self) -> u16 {
        let sum_hi = digit_weighted_sum(self.hi);
        let sum_lo = digit_weighted_sum(self.lo);
        let total = sum_hi + sum_lo;
        let check = (10u32 - (total % 10u32)) % 10u32;
        let digit = check as u16;
        self.digit = digit;
        digit
    }
}
