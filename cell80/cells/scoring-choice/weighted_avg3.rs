//! Normalized weighted mean of three inputs with caller-supplied weights: (a*wa + b*wb + c*wc) / (wa+wb+wc), 0 if the weights sum to zero — the normalized sibling of weighted_sum3, which returns the raw a*wa+b*wb+c*wc without dividing by the weight total.
//! tags: scoring, score, math, combine, weighted, average, mean, normalized, factor, checked
//! entry: WeightedAvg3::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if the running weighted sum overflows u32; returns 0 if wa+wb+wc == 0
struct WeightedAvg3 { a: u16, wa: u16, b: u16, wb: u16, c: u16, wc: u16, sum: u32 }
impl WeightedAvg3 {
    fn run(&mut self) -> u16 {
        let p1 = self.a as u32 * self.wa as u32;
        let p2 = self.b as u32 * self.wb as u32;
        let p3 = self.c as u32 * self.wc as u32;
        let s1 = add_checked_u32(p1, p2);
        let s2 = add_checked_u32(s1, p3);
        self.sum = s2;
        let denom = self.wa as u32 + self.wb as u32 + self.wc as u32;
        if denom == 0u32 { 0u16 } else { (s2 / denom) as u16 }
    }
}
