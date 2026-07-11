//! Given an arithmetic series' two endpoints (first, last) and its total sum, recovers the term count: count = 2*sum/(first+last) -- the missing inverse of series_sum (which only computes the sum, not the count that produced it).
//! tags: number, arithmetic, series, sequence, inverse, count, endpoints, first, last, sum, math, checked, wide, u32, escalate
//! entry: SeriesTermCount::run
//! limits: escalates (halt 0xFF06, out_of_domain) if first+last == 0 while sum != 0, or if 2*sum is not evenly divisible by (first+last); escalates (halt 0xFF05, needs_wider_math) if 2*sum overflows u32
struct SeriesTermCount { first: u32, last: u32, sum: u32, count: u32 }
impl SeriesTermCount {
    fn run(&mut self) -> u16 {
        let endpoint_sum = add_checked_u32(self.first, self.last);
        if endpoint_sum == 0u32 {
            if self.sum != 0u32 { halt(0xFF06u16); }
            self.count = 0u32;
            return 1u16;
        }
        let doubled = add_checked_u32(self.sum, self.sum);
        if doubled % endpoint_sum != 0u32 { halt(0xFF06u16); }
        self.count = doubled / endpoint_sum;
        1u16
    }
}
