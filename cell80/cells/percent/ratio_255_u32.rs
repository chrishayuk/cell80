//! Ratio scaled to a 0..255 byte fraction at wide u32 width: part*255/whole (0 if whole == 0), escalating (needs_wider_math) on multiply overflow rather than the u16 sibling's saturate-at-65535 behavior -- the wide sibling of ratio_255.
//! tags: ratio, byte, fraction, scale, proportion, normalize, wide, u32, checked, escalate
//! entry: Ratio255Wide::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if part*255 exceeds u32::MAX
struct Ratio255Wide { part: u32, whole: u32, result: u32 }
impl Ratio255Wide {
    fn run(&mut self) -> u16 {
        let p = mul_checked_u32(self.part, 255u32);
        let r = if self.whole != 0u32 { p / self.whole } else { 0u32 };
        self.result = r;
        1u16
    }
}
