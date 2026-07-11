//! Computes the Luhn check digit (0-9) to append to a partial 13-19 digit number split into hi/lo u32 decimal chunks (the same split luhn_check_u32 uses: lo is fixed at exactly the low 9 digits, hi holds everything above) -- the generate-side counterpart to that verify-only wide cell, mirroring luhn_check_digit at u32 width.
//! tags: checksum, luhn, check-digit, generate, card-number, mod10, wide, u32, hi-lo-split
//! entry: LuhnCheckDigitU32::run
//! limits: lo must hold exactly the low 9 decimal digits (0..=999999999); hi holds everything above that and must itself fit u32 (0..=4294967295), matching luhn_check_u32's own range
struct LuhnCheckDigitU32 { hi: u32, lo: u32, digit: u16 }
impl LuhnCheckDigitU32 {
    fn run(&mut self) -> u16 {
        let mut sum: u16 = 0u16;

        // lo carries the rightmost 9 digits, so its own position-0 digit is the
        // partial number's overall units digit -- same doubling parity as
        // luhn_check_digit's single-chunk loop (double when pos is even, since
        // appending the check digit shifts every partial digit one place right).
        let mut x = self.lo;
        let mut pos: u16 = 0u16;
        while x != 0u32 {
            let digit = (x % 10u32) as u16;
            let mut d = digit;
            if pos % 2u16 == 0u16 {
                d = d * 2u16;
                if d > 9u16 { d = d - 9u16; }
            }
            sum = sum + d;
            x = x / 10u32;
            pos = pos + 1u16;
        }

        // hi picks up where lo leaves off, at overall position 9 (odd), so hi's own
        // lowest digit (local position 0) must NOT double -- the opposite parity of
        // lo's, matching the hi/lo split luhn_check_u32 established.
        let mut y = self.hi;
        let mut hpos: u16 = 0u16;
        while y != 0u32 {
            let digit = (y % 10u32) as u16;
            let mut d = digit;
            if hpos % 2u16 == 1u16 {
                d = d * 2u16;
                if d > 9u16 { d = d - 9u16; }
            }
            sum = sum + d;
            y = y / 10u32;
            hpos = hpos + 1u16;
        }

        let result = (10u16 - (sum % 10u16)) % 10u16;
        self.digit = result;
        result
    }
}
