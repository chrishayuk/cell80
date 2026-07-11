//! Recover the starting value before N periods of compound bps increase, given the final value: loops v = v*10000/(10000+bps) exactly `periods` times -- the precise reverse of compound_increase_by_bps's own forward loop, completing the N-step backward quadrant original_before_bps_increase (single-step only) left open.
//! tags: money, bps, basis-points, original, reverse-percent, compound, compounding, periods, loop, checked, wide, u32
//! entry: CompoundOriginalBeforeIncrease::run
//! limits: escalates (halt 0xFF05, needs_wider_math) the moment any step's value * 10000 would overflow u32
struct CompoundOriginalBeforeIncrease { final_value: u32, bps: u32, periods: u16, original: u32 }
impl CompoundOriginalBeforeIncrease {
    fn run(&mut self) -> u16 {
        let denom = 10000u32 + self.bps;
        let mut v = self.final_value;
        let mut i = 0u16;
        while i < self.periods {
            let product = v.wrapping_mul(10000u32);
            if product / 10000u32 != v { halt(0xFF05u16); }
            v = product / denom;
            i = i + 1u16;
        }
        self.original = v;
        1u16
    }
}
