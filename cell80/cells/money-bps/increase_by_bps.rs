//! Increase a wide value by bps basis points (covers tax/tip/markup — same formula: value + value*bps/10000). Escalates on multiply or add overflow.
//! tags: money, bps, basis-points, tax, tip, markup, increase, checked, wide, u32
//! entry: IncreaseByBps::run
//! limits: escalates (halt 0xFF05, needs_wider_math) on overflow
struct IncreaseByBps { value: u32, bps: u32, result: u32 }
impl IncreaseByBps {
    fn run(&mut self) -> u16 {
        let product = mul_checked_u32(self.value, self.bps);
        let delta = product / 10000u32;
        let r = self.value + delta;
        if r < self.value { halt(0xFF05u16); }
        self.result = r;
        1u16
    }
}
