//! Returns 1 if n's decimal digits pass the Luhn checksum (mod 10, doubling every second digit from the right), else 0.
//! tags: checksum, luhn, validation, check-digit, card-number, mod10
//! limits: n is a u16 (up to 5 decimal digits) — a full 13-19 digit card number needs a wider (host-side) input
fn run(n: u16) -> u16 {
    let mut x = n;
    let mut pos: u16 = 0u16;
    let mut sum: u16 = 0u16;
    while x != 0u16 {
        let digit = x % 10u16;
        let mut d = digit;
        if pos % 2u16 == 1u16 {
            d = d * 2u16;
            if d > 9u16 { d = d - 9u16; }
        }
        sum = sum + d;
        x = x / 10u16;
        pos = pos + 1u16;
    }
    (sum % 10u16 == 0u16) as u16
}
