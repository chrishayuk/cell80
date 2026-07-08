//! Population covariance from precomputed sums (not a raw dataset -- that aggregation stays upstream, matching running_variance_step's own bivariate framing): cov = (n*sum_xy - sum_x*sum_y) / n^2, returned as an exact signed fraction (num/den, den always positive) rather than rounded to an integer.
//! tags: statistics, covariance, correlation, bivariate, mean, fraction, wide, u32, checked, escalate
//! entry: Covariance::run
//! limits: escalates (halt 0xFF06, out_of_domain) if n == 0; escalates (halt 0xFF05, needs_wider_math) if an intermediate product or sum overflows u32
struct Covariance { n: u32, sum_x: u32, sum_y: u32, sum_xy: u32, num_mag: u32, num_neg: u16, den: u32 }
impl Covariance {
    fn run(&mut self) -> u16 {
        if self.n == 0u32 { halt(0xFF06u16); }
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
        let den = mul_checked_u32(self.n, self.n);
        self.num_mag = num_mag;
        self.num_neg = num_neg;
        self.den = den;
        1u16
    }
}
