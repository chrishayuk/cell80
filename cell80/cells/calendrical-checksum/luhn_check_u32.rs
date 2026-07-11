//! Validates the Luhn checksum over a full 13-19 digit number split into two u32 decimal chunks (hi, lo where lo is fixed at exactly the low 9 digits) -- the wide sibling luhn_check's own docstring names as its gap (a full card number needs a wider input than one u16).
//! tags: checksum, luhn, validation, check-digit, card-number, mod10, wide, u32, large
//! entry: LuhnCheckU32::run
//! limits: lo must hold exactly the low 9 decimal digits (0..=999999999); hi holds everything above that and must itself fit u32 (0..=4294967295), which covers the standard 13-19 digit card-number range
struct LuhnCheckU32 { hi: u32, lo: u32, valid: u16 }
impl LuhnCheckU32 {
    fn run(&mut self) -> u16 {
        let mut sum: u16 = 0u16;

        // lo carries the rightmost 9 digits, so its own position-0 digit is the
        // number's overall units digit -- same doubling parity as luhn_check's
        // single-chunk loop (double when pos is odd).
        let mut x = self.lo;
        let mut pos: u16 = 0u16;
        while x != 0u32 {
            let digit = (x % 10u32) as u16;
            let mut d = digit;
            if pos % 2u16 == 1u16 {
                d = d * 2u16;
                if d > 9u16 { d = d - 9u16; }
            }
            sum = sum + d;
            x = x / 10u32;
            pos = pos + 1u16;
        }

        // hi picks up where lo leaves off, at overall position 9 (odd), so hi's own
        // lowest digit (local position 0) must double -- the opposite parity of lo's.
        let mut y = self.hi;
        let mut hpos: u16 = 0u16;
        while y != 0u32 {
            let digit = (y % 10u32) as u16;
            let mut d = digit;
            if hpos % 2u16 == 0u16 {
                d = d * 2u16;
                if d > 9u16 { d = d - 9u16; }
            }
            sum = sum + d;
            y = y / 10u32;
            hpos = hpos + 1u16;
        }

        let v = (sum % 10u16 == 0u16) as u16;
        self.valid = v;
        v
    }
}
