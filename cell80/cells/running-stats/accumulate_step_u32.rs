//! Wide/checked sibling of accumulate_step: running sum + count over a stream of u32 values, escalating (not saturating) on sum overflow via add_checked_u32 — the same escalate-on-overflow policy running_variance_step already uses for its own sum field, but without any of running_variance_step's variance/m2 machinery, for callers who just want a pure wide running total.
//! tags: running, sum, count, accumulate, stream, stats, mean, average, state, wide, u32, checked, escalate
//! entry: AccumulateU32::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if the running sum overflows u32
struct AccumulateU32 { value: u32, sum: u32, count: u32 }
impl AccumulateU32 {
    fn run(&mut self) -> u16 {
        self.sum = add_checked_u32(self.sum, self.value);
        self.count = self.count + 1u32;
        1u16
    }
}
