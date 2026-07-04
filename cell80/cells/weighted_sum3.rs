//! Weighted sum of three inputs with caller-supplied weights: a*wa + b*wb + c*wc. Sibling of weighted_sum/weighted_sum_wide (fixed weights 1, 2, 3) generalized to arbitrary weights, so a genuine u32 overflow is possible and escalates instead of silently wrapping.
//! tags: scoring, score, math, combine, weighted, factor, wide, checked
//! entry: WeightedSum3::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if the running sum overflows u32; the u16 return saturates at 65535 if the exact sum (the `sum` field) doesn't fit u16
struct WeightedSum3 { a: u16, wa: u16, b: u16, wb: u16, c: u16, wc: u16, sum: u32 }
impl WeightedSum3 {
    fn run(&mut self) -> u16 {
        let p1 = self.a as u32 * self.wa as u32;
        let p2 = self.b as u32 * self.wb as u32;
        let p3 = self.c as u32 * self.wc as u32;
        let s1 = p1.wrapping_add(p2);
        if s1 < p1 { halt(0xFF05u16); }
        let s2 = s1.wrapping_add(p3);
        if s2 < s1 { halt(0xFF05u16); }
        self.sum = s2;
        if (s2 >> 16u32) as u16 != 0u16 { 65535u16 } else { s2 as u16 }
    }
}
