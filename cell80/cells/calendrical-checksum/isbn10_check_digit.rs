//! Computes the ISBN-10 check character (0-9, or 10 meaning 'X') for a 9-digit body -- the generate-side counterpart to isbn10_check, weighting digits 10 down to 2 from the left and reducing mod 11.
//! tags: checksum, isbn, isbn10, check-digit, generate, mod11, book-number
//! entry: Isbn10CheckDigit::run
//! limits: body holds at most 9 decimal digits (fits u32); digits beyond the 9th (i.e. body >= 1_000_000_000) are ignored, only the low 9 decimal digits are weighted
struct Isbn10CheckDigit {
    body: u32,
    digit: u16,
}

impl Isbn10CheckDigit {
    fn run(&mut self) -> u16 {
        let mut x = self.body;
        let mut sum: u32 = 0u32;
        let mut weight: u32 = 2u32;
        let mut i: u32 = 0u32;
        while i < 9u32 {
            let d = x % 10u32;
            sum = sum + d * weight;
            x = x / 10u32;
            weight = weight + 1u32;
            i = i + 1u32;
        }
        let r = sum % 11u32;
        let check = (11u32 - r) % 11u32;
        let out = check as u16;
        self.digit = out;
        out
    }
}
