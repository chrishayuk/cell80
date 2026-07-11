//! Increase a wide value by pct percent: value + value*pct/100 at u32, escalating (needs_wider_math) on multiply or add overflow rather than the u16 sibling's saturate-at-65535 behavior -- the wide sibling of increase_percent.
//! tags: percent, increase, markup, raise, grow, surcharge, wide, u32, checked, escalate
//! entry: IncreasePercentWide::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if value*pct or the subsequent add overflows u32
struct IncreasePercentWide { value: u32, pct: u32, result: u32 }
impl IncreasePercentWide {
    fn run(&mut self) -> u16 {
        let p = mul_checked_u32(self.value, self.pct);
        let inc = p / 100u32;
        self.result = add_checked_u32(self.value, inc);
        1u16
    }
}
