//! Per-mille (parts per thousand) at wide u32 width: part*1000/whole (0 if whole == 0), escalating (needs_wider_math) on multiply overflow rather than the u16 sibling's saturate-at-65535 behavior -- the wide sibling of permille.
//! tags: percent, permille, thousandths, ratio, proportion, fraction, rate, wide, u32, checked, escalate
//! entry: PermilleWide::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if part*1000 exceeds u32::MAX
struct PermilleWide { part: u32, whole: u32, result: u32 }
impl PermilleWide {
    fn run(&mut self) -> u16 {
        let p = mul_checked_u32(self.part, 1000u32);
        let r = if self.whole != 0u32 { p / self.whole } else { 0u32 };
        self.result = r;
        1u16
    }
}
