//! Normalized weighted mean of two values with caller-supplied weights: (a*wa + b*wb) / (wa+wb), 0 if wa+wb==0 -- distinct from weighted_sum2, which returns the raw combined score a*wa + b*wb with no normalization by total weight.
//! tags: scoring, score, math, average, mean, weighted, normalize, blend, combine, ratio
//! entry: WeightedAvg2::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if a*wa + b*wb overflows u32
struct WeightedAvg2 { a: u16, wa: u16, b: u16, wb: u16, sum: u32 }
impl WeightedAvg2 {
    fn run(&mut self) -> u16 {
        let p1 = self.a as u32 * self.wa as u32;
        let p2 = self.b as u32 * self.wb as u32;
        let s = add_checked_u32(p1, p2);
        self.sum = s;
        let denom = self.wa as u32 + self.wb as u32;
        if denom == 0u32 { 0u16 } else { (s / denom) as u16 }
    }
}
