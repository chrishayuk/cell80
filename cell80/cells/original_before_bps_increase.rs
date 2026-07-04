//! Recover the original value before a bps increase, given the final value: final * 10000 / (10000 + bps). The inverse of increase_by_bps.
//! tags: money, bps, basis-points, original, reverse-percent, tax, markup, checked, wide, u32
//! entry: OriginalBeforeIncrease::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if final_value * 10000 overflows u32
struct OriginalBeforeIncrease { final_value: u32, bps: u32, original: u32 }
impl OriginalBeforeIncrease {
    fn run(&mut self) -> u16 {
        let denom = 10000u32 + self.bps;
        let product = self.final_value.wrapping_mul(10000u32);
        if product / 10000u32 != self.final_value { halt(0xFF05u16); }
        self.original = product / denom;
        1u16
    }
}
