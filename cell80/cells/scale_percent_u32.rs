//! Take pct percent of a wide value: value*pct/100 at u32, escalating if the multiply overflows — the wide sibling of scale_percent, and the percent-of core the widened (u32) arithmetic lane resolves to.
//! tags: percent, scale, of, fraction, proportion, multiply, wide, u32, checked, escalate
//! entry: ScalePercentWide::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if value*pct exceeds u32::MAX
struct ScalePercentWide { value: u32, pct: u32, result: u32 }
impl ScalePercentWide {
    fn run(&mut self) -> u16 {
        let p = mul_checked_u32(self.value, self.pct);
        self.result = p / 100u32;
        1u16
    }
}
