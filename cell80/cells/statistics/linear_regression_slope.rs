//! Ordinary-least-squares regression slope from precomputed sums (n, sum_x, sum_y, sum_xy, sum_x2 -- raw-dataset aggregation stays upstream): slope = (n*sum_xy - sum_x*sum_y) / (n*sum_x2 - sum_x^2), returned as an exact signed fraction rather than rounded. The denominator is n^2 times the population variance of x, which is always non-negative by construction -- zero only when every x value is identical (a vertical "line", no defined slope).
//! tags: statistics, regression, linear-regression, slope, ols, fit, fraction, wide, u32, checked, escalate
//! entry: LinearRegressionSlope::run
//! limits: escalates (halt 0xFF06, out_of_domain) if n == 0 or every x value is identical (undefined slope); escalates (halt 0xFF05, needs_wider_math) if an intermediate product or sum overflows u32
struct LinearRegressionSlope { n: u32, sum_x: u32, sum_y: u32, sum_xy: u32, sum_x2: u32, num_mag: u32, num_neg: u16, den: u32 }
impl LinearRegressionSlope {
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
        let d1 = mul_checked_u32(self.n, self.sum_x2);
        let d2 = mul_checked_u32(self.sum_x, self.sum_x);
        if d1 < d2 { halt(0xFF05u16); }
        let den = d1 - d2;
        if den == 0u32 { halt(0xFF06u16); }
        self.num_mag = num_mag;
        self.num_neg = num_neg;
        self.den = den;
        1u16
    }
}
