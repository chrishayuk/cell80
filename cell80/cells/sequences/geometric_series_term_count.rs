//! Given a geometric series' first term a, ratio r, and a target total target_sum, recover how many terms n must be summed to exactly reach it -- geometric_term_index already inverts a single term back to its index, but no cell inverted the running total back to a term count the way series_term_count does for arithmetic series, so this walks geometric_series_sum's own accumulate loop forward and halts the instant the running sum overshoots or growth stalls without ever matching.
//! tags: number, geometric, series, sequence, inverse, term, count, sum, ratio, math, checked, wide, u32, escalate
//! entry: GeometricSeriesTermCount::run
//! limits: escalates (halt 0xFF06, out_of_domain) if a == 0 while target_sum != 0, if the running sum overshoots target_sum without ever matching it exactly, or if growth stalls (ratio == 0) before matching; escalates (halt 0xFF05, needs_wider_math) if a term or the running sum overflows u32
struct GeometricSeriesTermCount { a: u32, r: u32, target_sum: u32, n: u32 }
impl GeometricSeriesTermCount {
    fn run(&mut self) -> u16 {
        if self.a == 0u32 {
            if self.target_sum == 0u32 {
                self.n = 0u32;
                return 1u16;
            }
            halt(0xFF06u16);
        }
        if self.r == 1u32 {
            if self.target_sum % self.a != 0u32 { halt(0xFF06u16); }
            self.n = self.target_sum / self.a;
            return 1u16;
        }
        let mut sum = 0u32;
        let mut term = self.a;
        let mut n = 0u32;
        while sum < self.target_sum {
            if term == 0u32 { halt(0xFF06u16); }
            let next_sum = add_checked_u32(sum, term);
            sum = next_sum;
            n = n + 1u32;
            if sum == self.target_sum {
                self.n = n;
                return 1u16;
            }
            if sum > self.target_sum { halt(0xFF06u16); }
            let next_term = mul_checked_u32(term, self.r);
            term = next_term;
        }
        self.n = n;
        1u16
    }
}
