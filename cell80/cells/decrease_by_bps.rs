//! Decrease a wide value by bps basis points (covers discount: value - value*bps/10000). Escalates if the discount would exceed the value, or on multiply overflow.
//! tags: money, bps, basis-points, discount, decrease, checked, wide, u32
//! entry: DecreaseByBps::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if bps > 10000 (or on overflow)
struct DecreaseByBps { value: u32, bps: u32, result: u32 }
impl DecreaseByBps {
    fn run(&mut self) -> u16 {
        let product = mul_checked_u32(self.value, self.bps);
        let delta = product / 10000u32;
        if delta > self.value { halt(0xFF05u16); }
        self.result = self.value - delta;
        1u16
    }
}
