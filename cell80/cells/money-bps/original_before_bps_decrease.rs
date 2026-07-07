//! Recover the original value before a bps decrease, given the final value: final * 10000 / (10000 - bps). The inverse of decrease_by_bps.
//! tags: money, bps, basis-points, original, reverse-percent, discount, checked, wide, u32
//! entry: OriginalBeforeDecrease::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if bps >= 10000, or if final_value * 10000 overflows u32
struct OriginalBeforeDecrease { final_value: u32, bps: u32, original: u32 }
impl OriginalBeforeDecrease {
    fn run(&mut self) -> u16 {
        if self.bps >= 10000u32 { halt(0xFF05u16); }
        let denom = 10000u32 - self.bps;
        let product = self.final_value.wrapping_mul(10000u32);
        if product / 10000u32 != self.final_value { halt(0xFF05u16); }
        self.original = product / denom;
        1u16
    }
}
