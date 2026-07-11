//! Number of digits of n when written in the given base (base >= 2; 0 has 1 digit) -- generalizes num_digits (decimal-only) with the same divide-until-zero counting loop run at an arbitrary base, the same base-parameter treatment is_palindromic_number gives palindrome checking.
//! tags: number, digits, length, base, radix, count, magnitude, generalized
//! limits: escalates (halt 0xFF06, out_of_domain) if base < 2
fn run(n: u16, base: u16) -> u16 {
    if base < 2u16 { halt(0xFF06u16); }
    let mut v = n;
    let mut c = 1u16;
    while v >= base { c = c + 1u16; v = v / base; }
    c
}
