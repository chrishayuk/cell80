//! Unbiased sample variance from precomputed sums (n, sum_x, sum_x2 -- raw-dataset aggregation stays upstream, the Bessel's-correction sibling of variance_from_sums): var = (n*sum_x2 - sum_x^2) / (n*(n-1)), returned as an exact non-negative fraction (num/den, den = n*(n-1)) rather than rounded to an integer.
//! tags: statistics, variance, sample, unbiased, bessel, univariate, mean, fraction, wide, u32, checked, escalate
//! entry: SampleVarianceFromSums::run
//! limits: escalates (halt 0xFF06, out_of_domain) if n < 2 (sample variance is undefined with fewer than two observations); escalates (halt 0xFF05, needs_wider_math) if an intermediate product overflows u32, or if n*sum_x2 < sum_x^2 (impossible for consistent sums of real data, so this signals corrupted or inconsistent inputs)
struct SampleVarianceFromSums { n: u32, sum_x: u32, sum_x2: u32, num: u32, den: u32 }
impl SampleVarianceFromSums {
    fn run(&mut self) -> u16 {
        if self.n < 2u32 { halt(0xFF06u16); }
        let d1 = mul_checked_u32(self.n, self.sum_x2);
        let d2 = mul_checked_u32(self.sum_x, self.sum_x);
        if d1 < d2 { halt(0xFF05u16); }
        let num = d1 - d2;
        let den = mul_checked_u32(self.n, self.n - 1u32);
        self.num = num;
        self.den = den;
        1u16
    }
}
