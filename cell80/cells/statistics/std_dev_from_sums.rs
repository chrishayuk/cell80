//! Population standard deviation from precomputed sums (n, sum_x, sum_x2 -- the precomputed-sums/batch sibling of running_stddev_step, and the sqrt-taking completion variance_from_sums stops short of): stddev = floor(sqrt((n*sum_x2 - sum_x^2) / n^2)), reusing variance_from_sums's checked num/den derivation then taking the integer square root of the truncated quotient via the same branch-free bitwise loop correlation.rs and effect_size_r.rs already inline for their own sqrt steps.
//! tags: statistics, stddev, standard-deviation, variance, univariate, batch, precomputed-sums, wide, u32, checked, escalate, sqrt
//! entry: StdDevFromSums::run
//! limits: escalates (halt 0xFF06, out_of_domain) if n == 0; escalates (halt 0xFF05, needs_wider_math) if n*sum_x2, sum_x*sum_x, or n*n overflows u32, or if n*sum_x2 < sum_x^2 (impossible for consistent sums of real data, so this signals corrupted or inconsistent inputs)
struct StdDevFromSums { n: u32, sum_x: u32, sum_x2: u32, stddev: u32 }
impl StdDevFromSums {
    fn run(&mut self) -> u16 {
        if self.n == 0u32 { halt(0xFF06u16); }
        let d1 = mul_checked_u32(self.n, self.sum_x2);
        let d2 = mul_checked_u32(self.sum_x, self.sum_x);
        if d1 < d2 { halt(0xFF05u16); }
        let num = d1 - d2;
        let den = mul_checked_u32(self.n, self.n);
        let variance = num / den;

        // Branch-free bitwise integer square root of `variance` directly (no Q8.8
        // scale-up: a raw u32 sqrt), the same loop correlation.rs and effect_size_r.rs
        // inline for their own sqrt steps, and running_stddev_step uses on its
        // running m2/count quotient.
        let mut val = variance;
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
        self.stddev = res;
        res as u16
    }
}
