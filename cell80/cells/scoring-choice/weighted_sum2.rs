//! Weighted sum of two inputs with caller-supplied weights: a*wa + b*wb (also known as score_2factor — the same formula under a different name). Sibling of weighted_sum/weighted_sum_wide (which use fixed weights 1, 2, 3), generalized to arbitrary weights, so a genuine u32 overflow is possible and escalates instead of silently wrapping.
//! tags: scoring, score, math, combine, weighted, score_2factor, factor, wide, checked
//! entry: WeightedSum2::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if a*wa + b*wb overflows u32; the u16 return saturates at 65535 if the exact sum (the `sum` field) doesn't fit u16
struct WeightedSum2 { a: u16, wa: u16, b: u16, wb: u16, sum: u32 }
impl WeightedSum2 {
    fn run(&mut self) -> u16 {
        let p1 = self.a as u32 * self.wa as u32;
        let p2 = self.b as u32 * self.wb as u32;
        let s = add_checked_u32(p1, p2);
        self.sum = s;
        if (s >> 16u32) as u16 != 0u16 { 65535u16 } else { s as u16 }
    }
}
