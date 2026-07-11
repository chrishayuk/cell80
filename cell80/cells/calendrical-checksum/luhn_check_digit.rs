//! Computes the Luhn check digit (0-9) to append to partial's digits so the completed number passes luhn_check — the generate-side counterpart to that verify-only cell.
//! tags: checksum, luhn, check-digit, generate, card-number, mod10
//! limits: partial is a u16 (up to 5 decimal digits) — a full 13-19 digit card number needs a wider (host-side) input
fn run(partial: u16) -> u16 {
    let mut x = partial;
    let mut pos: u16 = 0u16;
    let mut sum: u16 = 0u16;
    while x != 0u16 {
        let digit = x % 10u16;
        let mut d = digit;
        if pos % 2u16 == 0u16 {
            d = d * 2u16;
            if d > 9u16 { d = d - 9u16; }
        }
        sum = sum + d;
        x = x / 10u16;
        pos = pos + 1u16;
    }
    (10u16 - (sum % 10u16)) % 10u16
}
