//! Modular counter step: increments count by 1, wrapping to 0 the moment it would reach `limit` (limit 0 means never wrap — a plain saturating-free incrementer). Useful for round-robin dispatch or a bounded retry index. The caller threads `count` through — re-supply the field each call.
//! tags: counter, round-robin, cycle, wrap, index, dispatch, state, pick, next, worker
//! entry: CounterStep::run
struct CounterStep { count: u16, limit: u16 }
impl CounterStep {
    fn run(&mut self) -> u16 {
        let n = self.count + 1u16;
        let wrapped = if self.limit != 0u16 && n >= self.limit { 0u16 } else { n };
        self.count = wrapped;
        wrapped
    }
}
