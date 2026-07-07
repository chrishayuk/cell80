//! sigma_k(n): sum of the k-th powers of the positive divisors of n (n >= 1) -- generalizes factor_count (k=0, counts divisors) and sum_divisors (k=1, sums them) with an explicit exponent, the same general-parameter-sibling shape weighted_sum2 already gives weighted_sum.
//! tags: number, divisor, divisors, sum, power, sigma, exponent, generalized, wide, u32, checked, escalate, number-theory
//! entry: DivisorPowerSum::run
//! limits: escalates (halt 0xFF06, out_of_domain) if n == 0; escalates (halt 0xFF05, needs_wider_math) if any divisor's k-th power or the running sum overflows u32. d = 1 is special-cased to skip its own power loop (1^k = 1 for every k, so it never needs to iterate) -- without that, a caller passing a large k would burn a k-iteration loop computing a known-constant term.
struct DivisorPowerSum { n: u16, k: u16, result: u32 }
impl DivisorPowerSum {
    fn run(&mut self) -> u16 {
        if self.n == 0u16 { halt(0xFF06u16); }
        let mut sum = 0u32;
        let mut d = 1u16;
        while d < 256u16 && d * d <= self.n {
            if self.n % d == 0u16 {
                let mut dp = 1u32;
                if d != 1u16 {
                    let mut i = 0u16;
                    while i < self.k { dp = mul_checked_u32(dp, d as u32); i = i + 1u16; }
                }
                sum = add_checked_u32(sum, dp);
                let other = self.n / d;
                if other != d {
                    let mut op = 1u32;
                    let mut j = 0u16;
                    while j < self.k { op = mul_checked_u32(op, other as u32); j = j + 1u16; }
                    sum = add_checked_u32(sum, op);
                }
            }
            d = d + 1u16;
        }
        self.result = sum;
        1u16
    }
}
