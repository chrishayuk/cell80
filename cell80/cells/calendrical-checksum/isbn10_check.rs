//! Validates a full 10-character ISBN-10 via the mod-11 weighted checksum (weights 10 down to 1 across all ten positions, valid iff the total is divisible by 11): the 9-digit prefix packed as one u32 body plus a separate check value (0-9, or 10 for 'X') -- the full-number verify-only counterpart to luhn_check, for the ISBN-10 scheme instead of Luhn's mod-10.
//! tags: checksum, isbn, isbn10, validation, mod11, check-digit, book-number
//! entry: Isbn10Check::run
//! limits: assumes body packs at most 9 significant decimal digits (0..=999999999, leading zeros are fine since a zero digit contributes 0 at any weight); check is the 10th character's value (0-9, or 10 for 'X') -- check > 10 is treated as invalid (valid=0) rather than halting
struct Isbn10Check { body: u32, check: u16, valid: u16 }
impl Isbn10Check {
    fn run(&mut self) -> u16 {
        let mut x = self.body;
        let mut sum: u16 = 0u16;
        let mut weight: u16 = 2u16;
        let mut i = 0u16;
        while i < 9u16 {
            let digit = (x % 10u32) as u16;
            sum = sum + digit * weight;
            x = x / 10u32;
            weight = weight + 1u16;
            i = i + 1u16;
        }
        let v = if self.check <= 10u16 {
            let total = sum + self.check;
            (total % 11u16 == 0u16) as u16
        } else {
            0u16
        };
        self.valid = v;
        v
    }
}
