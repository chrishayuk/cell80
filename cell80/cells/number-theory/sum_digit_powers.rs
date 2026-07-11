//! Sum of each decimal digit of n raised to power p: sum(digit_i^p) -- generalizes digit_sum (p=1) with an explicit exponent, the same general-parameter-sibling shape jordan_totient gives euler_totient and divisor_power_sum gives sum_divisors. Each digit's p-th power is built by repeated checked multiplication (mul_checked_u32) rather than a call to pow_small (a u32 value can't cross a call boundary), and the running sum is accumulated at u32 width via add_checked_u32 -- both the per-digit term and the running sum are guarded, so nothing silently wraps.
//! tags: number, digit, digits, power, exponent, sum, generalized, decimal, wide, u32, checked, escalate, number-theory
//! limits: escalates (halt 0xFF05, needs_wider_math) if any digit's p-th power overflows u32, the running sum overflows u32, or the final total exceeds 65535
fn run(n: u16, p: u16) -> u16 {
    let mut v = n;
    let mut sum = 0u32;
    while v != 0u16 {
        let digit = v % 10u16;
        let mut term = 1u32;
        let mut i = 0u16;
        while i < p {
            term = mul_checked_u32(term, digit as u32);
            i = i + 1u16;
        }
        sum = add_checked_u32(sum, term);
        v = v / 10u16;
    }
    if sum > 65535u32 { halt(0xFF05u16); }
    sum as u16
}
