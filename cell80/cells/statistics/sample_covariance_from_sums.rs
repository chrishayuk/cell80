//! Unbiased sample covariance from precomputed sums (n, sum_x, sum_y, sum_xy -- raw-dataset aggregation stays upstream, the Bessel's-correction sibling of covariance's population framing): cov = (n*sum_xy - sum_x*sum_y) / (n*(n-1)), returned as an exact signed fraction (num_mag/den, num_neg 0=nonnegative/1=negative) rather than rounded to an integer.
//! tags: statistics, covariance, sample, unbiased, bessel, bivariate, mean, fraction, wide, u32, checked, escalate
//! entry: SampleCovarianceFromSums::run
//! limits: escalates (halt 0xFF06, out_of_domain) if n < 2 (sample covariance is undefined with fewer than two observations); escalates (halt 0xFF05, needs_wider_math) if an intermediate product overflows u32
struct SampleCovarianceFromSums { n: u32, sum_x: u32, sum_y: u32, sum_xy: u32, num_mag: u32, num_neg: u16, den: u32 }
impl SampleCovarianceFromSums {
    fn run(&mut self) -> u16 {
        if self.n < 2u32 { halt(0xFF06u16); }
        let p1 = mul_checked_u32(self.n, self.sum_xy);
        let p2 = mul_checked_u32(self.sum_x, self.sum_y);
        let mut num_mag = 0u32;
        let mut num_neg = 0u16;
        if p1 >= p2 {
            num_mag = p1 - p2;
        } else {
            num_mag = p2 - p1;
            num_neg = 1u16;
        }
        let den = mul_checked_u32(self.n, self.n - 1u32);
        self.num_mag = num_mag;
        self.num_neg = num_neg;
        self.den = den;
        1u16
    }
}
