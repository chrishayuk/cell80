//! Infer the basis-points decrease between two wide values: given before and after (after <= before), the rate = (before - after) * 10000 / before — the inverse of decrease_by_bps (that computes the final value from a rate; this recovers the rate from the two values).
//! tags: money, bps, basis-points, percent, decrease, discount, rate, wide, u32, checked, escalate
//! entry: BpsDecreaseBetween::run
//! limits: escalates (halt 0xFF06, out_of_domain) if before == 0 or after > before; escalates (halt 0xFF05, needs_wider_math) if the multiply overflows u32
struct BpsDecreaseBetween { before: u32, after: u32, bps: u32 }
impl BpsDecreaseBetween {
    fn run(&mut self) -> u16 {
        if self.before == 0u32 || self.after > self.before { halt(0xFF06u16); }
        let diff = self.before - self.after;
        let scaled = mul_checked_u32(diff, 10000u32);
        self.bps = scaled / self.before;
        1u16
    }
}
