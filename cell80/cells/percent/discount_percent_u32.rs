//! Decrease a wide value by pct percent: value - value*pct/100 (0 if pct >= 100) at u32 width — wide sibling of discount_percent, using a checked multiply for the intermediate product.
//! tags: percent, discount, decrease, reduce, markdown, off, wide, u32, checked
//! entry: DiscountPercentWide::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if value*pct exceeds u32::MAX
struct DiscountPercentWide { value: u32, pct: u32, result: u32 }
impl DiscountPercentWide {
    fn run(&mut self) -> u16 {
        let product = mul_checked_u32(self.value, self.pct);
        let delta = product / 100u32;
        let r = if self.pct < 100u32 { self.value - delta } else { 0u32 };
        self.result = r;
        1u16
    }
}
