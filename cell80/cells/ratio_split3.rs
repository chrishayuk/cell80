//! Split a wide total three ways by a given ratio (ratio_a : ratio_b : ratio_c): part_a and part_b get their proportional share by integer division, part_c takes the remainder — guaranteed to sum exactly to total (the direct 3-way sibling of ratio_split2).
//! tags: fraction, frac, ratio, split, divide, three, wide, u32, checked, escalate
//! entry: RatioSplit3::run
//! limits: escalates (halt 0xFF06, out_of_domain) if ratio_a + ratio_b + ratio_c == 0; escalates (halt 0xFF05, needs_wider_math) if total * ratio_a or total * ratio_b overflows u32
struct RatioSplit3 { total: u32, ratio_a: u32, ratio_b: u32, ratio_c: u32, part_a: u32, part_b: u32, part_c: u32 }
impl RatioSplit3 {
    fn run(&mut self) -> u16 {
        let denom = self.ratio_a.wrapping_add(self.ratio_b).wrapping_add(self.ratio_c);
        if denom == 0u32 { halt(0xFF06u16); }
        let pa_num = self.total.wrapping_mul(self.ratio_a);
        if self.total != 0u32 && pa_num / self.total != self.ratio_a { halt(0xFF05u16); }
        let pb_num = self.total.wrapping_mul(self.ratio_b);
        if self.total != 0u32 && pb_num / self.total != self.ratio_b { halt(0xFF05u16); }
        self.part_a = pa_num / denom;
        self.part_b = pb_num / denom;
        self.part_c = self.total - self.part_a - self.part_b;
        1u16
    }
}
