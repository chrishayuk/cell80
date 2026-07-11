//! Apply the same bps decrease rate repeatedly for `periods` iterations (value -= value*bps/10000 each step), covering depreciation/decay compounding — distinct from decrease_by_bps, which only ever applies the discount once.
//! tags: money, bps, basis-points, discount, decrease, compound, depreciation, decay, checked, wide, u32
//! entry: CompoundDecreaseByBps::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if any step's discount would exceed the running value, or on multiply overflow
struct CompoundDecreaseByBps { value: u32, bps: u32, periods: u16, result: u32 }
impl CompoundDecreaseByBps {
    fn run(&mut self) -> u16 {
        let mut v = self.value;
        let mut i = 0u16;
        while i < self.periods {
            let product = mul_checked_u32(v, self.bps);
            let delta = product / 10000u32;
            if delta > v { halt(0xFF05u16); }
            v = v - delta;
            i = i + 1u16;
        }
        self.result = v;
        1u16
    }
}
