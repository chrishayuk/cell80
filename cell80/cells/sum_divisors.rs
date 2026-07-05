//! Sum of the positive divisors of n (n >= 1), including 1 and n itself (sigma(n)) — the sum-valued sibling of factor_count (which counts divisors; this sums them, so it needs a wide result field since sigma(n) routinely exceeds 65535 within the u16 domain).
//! tags: number, divisor, divisors, sum, sigma, factor, wide, u32, number-theory
//! entry: SumDivisors::run
//! limits: escalates (halt 0xFF06, out_of_domain) if n == 0
struct SumDivisors { n: u16, result: u32 }
impl SumDivisors {
    fn run(&mut self) -> u16 {
        if self.n == 0u16 { halt(0xFF06u16); }
        let mut sum = 0u32;
        let mut d = 1u16;
        while d < 256u16 && d * d <= self.n {
            if self.n % d == 0u16 {
                sum = sum + d as u32;
                let other = self.n / d;
                if other != d { sum = sum + other as u32; }
            }
            d = d + 1u16;
        }
        self.result = sum;
        1u16
    }
}
