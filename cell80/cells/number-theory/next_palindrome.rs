//! The smallest decimal palindrome strictly greater than n. Searches upward candidate by candidate (the worst-case gap within the u16 domain is 110, at n=1001 -- cheap); escalates if no palindrome exists at or below 65535 (true for n in roughly [65456, 65535], where reaching one would need a 6th digit).
//! tags: number, palindrome, palindromic, next, digit, search, math
//! limits: escalates (halt 0xFF05, needs_wider_math) if no palindrome greater than n fits in u16
fn run(n: u16) -> u16 {
    let mut c = n as u32 + 1u32;
    while c <= 65535u32 {
        let mut v = c as u16;
        let mut r = 0u32;
        while v != 0u16 {
            r = r * 10u32 + (v % 10u16) as u32;
            v = v / 10u16;
        }
        if r == c {
            return c as u16;
        }
        c = c + 1u32;
    }
    halt(0xFF05u16);
    0u16
}
