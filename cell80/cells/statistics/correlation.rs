//! Pearson correlation coefficient from precomputed sums (n, sum_x, sum_y, sum_xy, sum_x2, sum_y2 -- raw-dataset aggregation stays upstream), returned as a Q8.8 fixed-point value bounded to [-1, 1] by construction (Cauchy-Schwarz). r = (n*sum_xy - sum_x*sum_y) / sqrt((n*sum_x2 - sum_x^2) * (n*sum_y2 - sum_y^2)) -- the numerator signed, the two variance-like factors always non-negative, their product's integer square root taken at a 256x-scaled precision (the same order effect_size_r uses: scale up, sqrt once, divide last).
//! tags: statistics, correlation, pearson, coefficient, bivariate, fixed-point, q8.8, wide, u32, checked, escalate
//! entry: Correlation::run
//! scale: 8
//! limits: escalates (halt 0xFF06, out_of_domain) if n == 0 or either variance-like factor's product is zero (no variance in x or y -- correlation undefined); escalates (halt 0xFF05, needs_wider_math) if any intermediate product, including the scaled variance product, overflows u32 -- realistic small-to-moderate datasets stay well inside this bound, but the denominator being a *product* of two sums-of-squares-like quantities makes this a narrower safe domain than effect_size_r's
fn isqrt_u32(n: u32) -> u32 {
    let mut val = n;
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
    res
}
struct Correlation {
    n: u32, sum_x: u32, sum_y: u32, sum_xy: u32, sum_x2: u32, sum_y2: u32,
    r_mag: u16, r_neg: u16
}
impl Correlation {
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
        let f1 = d1 - d2;

        let d3 = mul_checked_u32(self.n, self.sum_y2);
        let d4 = mul_checked_u32(self.sum_y, self.sum_y);
        if d3 < d4 { halt(0xFF05u16); }
        let f2 = d3 - d4;

        let denom_sq = mul_checked_u32(f1, f2);
        if denom_sq == 0u32 { halt(0xFF06u16); }
        let scaled = mul_checked_u32(denom_sq, 256u32);
        let s2 = isqrt_u32(scaled);
        if s2 == 0u32 { halt(0xFF06u16); }

        let num_scaled = mul_checked_u32(num_mag, 4096u32);
        let mut r_q8 = num_scaled / s2;
        if r_q8 > 256u32 { r_q8 = 256u32; }
        self.r_mag = r_q8 as u16;
        self.r_neg = num_neg;
        1u16
    }
}
