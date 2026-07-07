//! Check whether n is palindromic when written in the given base (base >= 2) -- its digits read the same forwards and backwards. Computed by reversing n's base-b digits (the same trick digit_reverse uses at base 10) and comparing to the original, rather than building a digit array.
//! tags: number, palindrome, palindromic, digit, base, radix, predicate, math
//! limits: escalates (halt 0xFF06, out_of_domain) if base < 2; escalates (halt 0xFF05, needs_wider_math) if the reversal would exceed 65535
fn run(n: u16, base: u16) -> u16 {
    if base < 2u16 { halt(0xFF06u16); }
    let mut v = n;
    let mut r = 0u32;
    while v != 0u16 {
        r = r * base as u32 + (v % base) as u32;
        v = v / base;
    }
    if r > 65535u32 { halt(0xFF05u16); }
    (r as u16 == n) as u16
}
