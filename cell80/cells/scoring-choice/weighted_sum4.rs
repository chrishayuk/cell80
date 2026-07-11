//! Weighted sum of four inputs with caller-supplied weights: a*wa + b*wb + c*wc + d*wd. Sibling of weighted_sum2/weighted_sum3 (arbitrary weights) extended to a fourth operand, which weighted_sum_wide does not offer (it only widens the fixed-weight 3-operand cell's output, not the operand count or the weights).
//! tags: scoring, score, math, combine, weighted, factor, wide, checked
//! entry: WeightedSum4::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if the running sum overflows u32; the u16 return saturates at 65535 if the exact sum (the `sum` field) doesn't fit u16
struct WeightedSum4 { a: u16, wa: u16, b: u16, wb: u16, c: u16, wc: u16, d: u16, wd: u16, sum: u32 }
impl WeightedSum4 {
    fn run(&mut self) -> u16 {
        let p1 = self.a as u32 * self.wa as u32;
        let p2 = self.b as u32 * self.wb as u32;
        let p3 = self.c as u32 * self.wc as u32;
        let p4 = self.d as u32 * self.wd as u32;
        let s1 = add_checked_u32(p1, p2);
        let s2 = add_checked_u32(s1, p3);
        let s3 = add_checked_u32(s2, p4);
        self.sum = s3;
        if (s3 >> 16u32) as u16 != 0u16 { 65535u16 } else { s3 as u16 }
    }
}
