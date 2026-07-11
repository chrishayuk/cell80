//! Given an arithmetic series' term count, sum, and one known endpoint, recovers the other endpoint: other = 2*sum/count - known -- the missing endpoint inverse (series_sum takes both endpoints to a sum, series_term_count inverts for count, neither recovers a single endpoint).
//! tags: number, arithmetic, series, sequence, inverse, endpoint, first, last, count, sum, math, checked, wide, u32, escalate
//! entry: SeriesOtherEndpoint::run
//! limits: escalates (halt 0xFF06, out_of_domain) if count == 0, if 2*sum is not evenly divisible by count, or if known exceeds the recovered endpoint average; escalates (halt 0xFF05, needs_wider_math) if 2*sum overflows u32
struct SeriesOtherEndpoint { known: u32, count: u32, sum: u32, other: u32 }
impl SeriesOtherEndpoint {
    fn run(&mut self) -> u16 {
        if self.count == 0u32 { halt(0xFF06u16); }
        let doubled = add_checked_u32(self.sum, self.sum);
        if doubled % self.count != 0u32 { halt(0xFF06u16); }
        let avg_pair = doubled / self.count;
        if self.known > avg_pair { halt(0xFF06u16); }
        self.other = avg_pair - self.known;
        1u16
    }
}
