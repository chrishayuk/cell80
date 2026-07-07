//! Check whether every decimal digit of n is the same digit (e.g. 4444, 555, 22 -- and trivially any single digit 0-9). Distinct from is_palindromic_number: a repdigit is always a palindrome but not vice versa (121 is palindromic, not a repdigit).
//! tags: number, repdigit, digit, repeated, uniform, predicate, math
fn run(n: u16) -> u16 {
    let mut v = n;
    let d = v % 10u16;
    let mut ok = 1u16;
    while v != 0u16 {
        if v % 10u16 != d { ok = 0u16; }
        v = v / 10u16;
    }
    ok
}
