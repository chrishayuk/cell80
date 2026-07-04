//! Split a wide total into two parts in a given ratio (ratio_a : ratio_b): part_a = total*ratio_a/(ratio_a+ratio_b), part_b = total - part_a — guaranteed to sum exactly to total (the remainder from integer division always lands on part_b), unlike computing both parts independently.
//! tags: ratio, split, fraction, frac, proportion, wide, u32, checked
//! entry: RatioSplit2::run
//! limits: escalates (halt 0xFF06, out_of_domain) if ratio_a + ratio_b == 0; escalates (halt 0xFF05, needs_wider_math) if total * ratio_a overflows u32
struct RatioSplit2 { total: u32, ratio_a: u32, ratio_b: u32, part_a: u32, part_b: u32 }
impl RatioSplit2 {
    fn run(&mut self) -> u16 {
        let sum = self.ratio_a.wrapping_add(self.ratio_b);
        if sum == 0u32 { halt(0xFF06u16); }
        let product = self.total.wrapping_mul(self.ratio_a);
        if self.total != 0u32 && product / self.total != self.ratio_a { halt(0xFF05u16); }
        self.part_a = product / sum;
        self.part_b = self.total - self.part_a;
        1u16
    }
}
