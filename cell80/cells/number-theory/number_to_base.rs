//! Convert n to its base-b digit expansion (2 <= base <= 9) and reassemble those digits, in the same left-to-right order, as a base-10 display number (e.g. n=13, base=2 -> binary "1101" -> the decimal literal 1101) -- distinct from bcd_encode/bcd_encode16 (pack decimal digits into nibbles, no base conversion) and from is_palindromic_number/num_digits_base (only compare/count base-b digits internally, never expose or reassemble them). Digits are extracted least-significant-first via the usual mod/div loop and folded into a u32 accumulator with an increasing power-of-ten place value, so each digit lands at the same positional slot in the base-10 result that it held in the base-b expansion -- no reversal needed, unlike digit_reverse/is_palindromic_number's r = r*base + digit trick, which deliberately reverses.
//! tags: number, base, radix, convert, digits, reassemble, expansion, display, wide, u32, checked, escalate, number-theory
//! limits: escalates (halt 0xFF05, needs_wider_math) if base is outside [2,9] or the reassembled value would overflow u16 (or overflow u32 along the way)
fn run(n: u16, base: u16) -> u16 {
    if base < 2u16 || base > 9u16 { halt(0xFF05u16); }
    let mut v = n;
    let mut result = 0u32;
    let mut place = 1u32;
    while v != 0u16 {
        let d = (v % base) as u32;
        let term = mul_checked_u32(d, place);
        result = add_checked_u32(result, term);
        v = v / base;
        place = mul_checked_u32(place, 10u32);
    }
    if result > 65535u32 { halt(0xFF05u16); }
    result as u16
}
