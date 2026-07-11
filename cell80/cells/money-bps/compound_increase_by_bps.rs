//! Apply the same bps increase rate repeatedly for `periods` iterations (value += value*bps/10000 each step, e.g. compound interest or repeated markup) -- distinct from increase_by_bps's single application, this loops the same rate N times.
//! tags: money, bps, basis-points, compound, compounding, interest, markup, periods, loop, checked, wide, u32
//! entry: CompoundIncreaseByBps::run
//! limits: escalates (halt 0xFF05, needs_wider_math) the moment any step's multiply or add would overflow u32
struct CompoundIncreaseByBps { value: u32, bps: u32, periods: u16, result: u32 }
impl CompoundIncreaseByBps {
    fn run(&mut self) -> u16 {
        let mut v = self.value;
        let mut i = 0u16;
        while i < self.periods {
            let product = mul_checked_u32(v, self.bps);
            let delta = product / 10000u32;
            let r = add_checked_u32(v, delta);
            v = r;
            i = i + 1u16;
        }
        self.result = v;
        1u16
    }
}
