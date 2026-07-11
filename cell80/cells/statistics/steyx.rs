//! Standard error of the y-estimate for a linear regression from precomputed sums (n, sum_x, sum_y, sum_xy, sum_x2, sum_y2 -- the exact six sums correlation.rs already consumes, raw-dataset aggregation stays upstream): steyx = sqrt(SSE/(n-2)) where SSE = Syy - Sxy^2/Sxx, computed here entirely over n-scaled integer quantities (Sxx*n, Syy*n, Sxy*n -- the same numerators linear_regression_slope/intercept already derive) as SSE = (Sxx_n*Syy_n - Sxy_n^2) / (n*Sxx_n), then divided by (n-2) before the truncated quotient's integer square root is taken via the same branch-free bitwise loop correlation.rs and std_dev_from_sums.rs already inline -- distinct from both: correlation stops at a bounded [-1,1] ratio and std_dev_from_sums never touches a second variable y or a degrees-of-freedom correction at all.
//! tags: statistics, regression, linear-regression, steyx, standard-error, residual, sse, ols, fit, degrees-of-freedom, wide, u32, checked, escalate, sqrt
//! entry: Steyx::run
//! limits: escalates (halt 0xFF06, out_of_domain) if n <= 2 (fewer than 3 points leaves zero or negative degrees of freedom, n-2, undefined) or if every x value is identical (Sxx*n == 0, the same undefined-regression condition linear_regression_slope/intercept already check); escalates (halt 0xFF05, needs_wider_math) if any intermediate product overflows u32, or if Sxx_n*Syy_n < Sxy_n^2 -- impossible for consistent sums of real data by Cauchy-Schwarz (the same invariant correlation.rs's bounded ratio relies on), so this signals corrupted or inconsistent input sums
struct Steyx {
    n: u32,
    sum_x: u32,
    sum_y: u32,
    sum_xy: u32,
    sum_x2: u32,
    sum_y2: u32,
    steyx: u32,
}
impl Steyx {
    fn run(&mut self) -> u16 {
        if self.n <= 2u32 { halt(0xFF06u16); }

        let d1 = mul_checked_u32(self.n, self.sum_x2);
        let d2 = mul_checked_u32(self.sum_x, self.sum_x);
        if d1 < d2 { halt(0xFF05u16); }
        let sxx_n = d1 - d2;
        if sxx_n == 0u32 { halt(0xFF06u16); }

        let d3 = mul_checked_u32(self.n, self.sum_y2);
        let d4 = mul_checked_u32(self.sum_y, self.sum_y);
        if d3 < d4 { halt(0xFF05u16); }
        let syy_n = d3 - d4;

        let p1 = mul_checked_u32(self.n, self.sum_xy);
        let p2 = mul_checked_u32(self.sum_x, self.sum_y);
        let mut sxy_n_mag = 0u32;
        if p1 >= p2 {
            sxy_n_mag = p1 - p2;
        } else {
            sxy_n_mag = p2 - p1;
        }

        let ac = mul_checked_u32(sxx_n, syy_n);
        let b2 = mul_checked_u32(sxy_n_mag, sxy_n_mag);
        if ac < b2 { halt(0xFF05u16); }
        let sse_num = ac - b2;

        let n_minus_2 = self.n - 2u32;
        let n_a = mul_checked_u32(self.n, sxx_n);
        let denom = mul_checked_u32(n_a, n_minus_2);

        let quotient = sse_num / denom;

        // Branch-free bitwise integer square root of the truncated quotient directly
        // (no Q8.8 scale-up), the same loop correlation.rs and std_dev_from_sums.rs
        // each inline for their own sqrt steps.
        let mut val = quotient;
        let mut res = 0u32;
        let mut bit = 1u32 << 30u32;
        while bit > val { bit = bit >> 2u32; }
        while bit != 0u32 {
            if val >= res + bit {
                val = val - (res + bit);
                res = (res >> 1u32) + bit;
            } else {
                res = res >> 1u32;
            }
            bit = bit >> 2u32;
        }
        self.steyx = res;
        res as u16
    }
}
